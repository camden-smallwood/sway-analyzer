use crate::{
    error::Error,
    project::Project,
    report::Severity,
    utils,
    visitor::{
        AstVisitor, ConfigurableFieldContext, ConstContext, EnumFieldContext, ExprContext,
        FnContext, ModuleContext, StatementLetContext, StorageFieldContext, StructFieldContext,
        TraitContext, TraitTypeContext, TypeAliasContext, UseContext,
    },
};
use std::{collections::HashMap, path::PathBuf};
use sway_ast::{
    ty::TyTupleDescriptor, Expr, FnArgs, PathExpr, PathType, Pattern, Traits, Ty, UseTree,
};
use sway_types::{Span, Spanned};

#[derive(Default)]
pub struct UnusedImportVisitor {
    module_states: HashMap<PathBuf, ModuleState>,
}

#[derive(Default)]
struct ModuleState {
    usage_states: HashMap<Span, u32>,
}

impl ModuleState {
    fn import_use_tree(&mut self, use_tree: &UseTree) {
        match use_tree {
            UseTree::Group { imports } => {
                for use_tree in &imports.inner {
                    self.import_use_tree(use_tree);
                }
            }

            UseTree::Name { name } => {
                self.usage_states.insert(name.span(), 0);
            }

            UseTree::Rename { alias, .. } => {
                self.usage_states.insert(alias.span(), 0);
            }
            
            UseTree::Glob { .. } => {}

            UseTree::Path { suffix, .. } => {
                self.import_use_tree(suffix.as_ref());
            }

            UseTree::Error { .. } => {}
        }
    }

    fn check_span_usage(&mut self, span: &Span) {
        let Some((_, usage_state)) = self.usage_states.iter_mut().find(|(s, _)| s.as_str() == span.as_str()) else { return };
        *usage_state += 1;
    }

    fn check_expr_usage(&mut self, expr: &Expr) {
        utils::map_expr(expr, &mut |expr| {
            if let Expr::Path(PathExpr { prefix, .. }) = expr {
                self.check_span_usage(&prefix.span());
            }
        });
    }

    fn check_pattern_usage(&mut self, pattern: &Pattern) {
        utils::map_pattern(pattern, &mut |pattern| {
            if let Pattern::Constructor { path, .. } | Pattern::Struct { path, .. } = pattern {
                self.check_span_usage(&path.span());
            }
        });
    }

    fn check_path_type_usage(&mut self, path: &PathType) {
        self.check_span_usage(&path.prefix.name.span());
                
        if let Some(generics) = path.prefix.generics_opt.as_ref() {
            for ty in &generics.1.parameters.inner {
                self.check_ty_usage(ty);
            }
        }

        if let Some(root) = path.root_opt.as_ref() {
            if let Some(root) = root.0.as_ref() {
                self.check_ty_usage(root.inner.ty.as_ref());
                
                if let Some(as_trait) = root.inner.as_trait.as_ref() {
                    self.check_path_type_usage(as_trait.1.as_ref());
                }
            }
        }
    }

    fn check_ty_usage(&mut self, ty: &Ty) {
        match ty {
            Ty::Path(path) => {
                self.check_path_type_usage(path);
            }

            Ty::Tuple(tuple) => {
                if let TyTupleDescriptor::Cons { head, tail, .. } = &tuple.inner {
                    self.check_ty_usage(head.as_ref());
                    
                    for ty in tail {
                        self.check_ty_usage(ty);
                    }
                }
            }

            Ty::Array(array) => {
                self.check_ty_usage(&array.inner.ty);
                self.check_expr_usage(array.inner.length.as_ref());
            }
            
            Ty::StringSlice(_) => {}
            Ty::StringArray { .. } => {}
            Ty::Infer { .. } => {}
            
            Ty::Ptr { ty, .. } |
            Ty::Slice { ty, .. } => {
                self.check_ty_usage(ty.inner.as_ref());
            }
        }
    }
}

impl AstVisitor for UnusedImportVisitor {
    fn visit_module(&mut self, context: &ModuleContext, _project: &mut Project) -> Result<(), Error> {
        if !self.module_states.contains_key(context.path) {
            self.module_states.insert(context.path.into(), ModuleState::default());
        }

        Ok(())
    }

    fn leave_module(&mut self, context: &ModuleContext, project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        for (span, count) in &module_state.usage_states {
            if *count == 0 {
                project.report.borrow_mut().add_entry(
                    context.path,
                    project.span_to_line(context.path, span)?,
                    Severity::Low,
                    format!(
                        "Found unused import: `{}`. Consider removing any unused imports.",
                        span.as_str(),
                    ),
                );
            }
        }

        Ok(())
    }

    fn visit_use(&mut self, context: &UseContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.import_use_tree(&context.item_use.tree);
        
        Ok(())
    }

    fn visit_struct_field(&mut self, context: &StructFieldContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.check_ty_usage(&context.field.ty);

        Ok(())
    }

    fn visit_enum_field(&mut self, context: &EnumFieldContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.check_ty_usage(&context.field.ty);

        Ok(())
    }

    fn visit_fn(&mut self, context: &FnContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        if let Some(where_clause) = context.item_fn.fn_signature.where_clause_opt.as_ref() {
            for bound in &where_clause.bounds {
                module_state.check_path_type_usage(&bound.bounds.prefix);
    
                for (_, path_type) in bound.bounds.suffixes.iter() {
                    module_state.check_path_type_usage(path_type);
                }
            }
        }

        let args = match &context.item_fn.fn_signature.arguments.inner {
            FnArgs::Static(args) => args,
            FnArgs::NonStatic { args_opt: Some(args), .. } => &args.1,
            _ => return Ok(()),
        };

        for arg in args {
            module_state.check_pattern_usage(&arg.pattern);
            module_state.check_ty_usage(&arg.ty);
        }

        Ok(())
    }

    fn visit_statement_let(&mut self, context: &StatementLetContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        module_state.check_pattern_usage(&context.statement_let.pattern);

        if let Some((_, ty)) = context.statement_let.ty_opt.as_ref() {
            module_state.check_ty_usage(ty);
        }

        module_state.check_expr_usage(&context.statement_let.expr);

        Ok(())
    }

    fn visit_expr(&mut self, context: &ExprContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.check_expr_usage(context.expr);

        Ok(())
    }

    fn visit_trait(&mut self, context: &TraitContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        let mut check_traits = |traits: &Traits| {
            module_state.check_path_type_usage(&traits.prefix);

            for (_, path_type) in traits.suffixes.iter() {
                module_state.check_path_type_usage(path_type);
            }
        };

        if let Some((_, super_traits)) = context.item_trait.super_traits.as_ref() {
            check_traits(super_traits);
        }

        if let Some(where_clause) = context.item_trait.where_clause_opt.as_ref() {
            for bound in &where_clause.bounds {
                check_traits(&bound.bounds);
            }
        }

        Ok(())
    }

    fn visit_const(&mut self, context: &ConstContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        if let Some((_, ty)) = context.item_const.ty_opt.as_ref() {
            module_state.check_ty_usage(ty);
        }

        if let Some(expr) = context.item_const.expr_opt.as_ref() {
            module_state.check_expr_usage(expr);
        }

        Ok(())
    }

    fn visit_storage_field(&mut self, context: &StorageFieldContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();

        module_state.check_ty_usage(&context.field.ty);
        module_state.check_expr_usage(&context.field.initializer);
        
        Ok(())
    }

    fn visit_configurable_field(&mut self, context: &ConfigurableFieldContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.check_ty_usage(&context.field.ty);
        module_state.check_expr_usage(&context.field.initializer);

        Ok(())
    }

    fn visit_type_alias(&mut self, context: &TypeAliasContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        module_state.check_ty_usage(&context.item_type_alias.ty);
        
        Ok(())
    }

    fn visit_trait_type(&mut self, context: &TraitTypeContext, _project: &mut Project) -> Result<(), Error> {
        let module_state = self.module_states.get_mut(context.path).unwrap();
        
        if let Some(ty) = context.item_type.ty_opt.as_ref() {
            module_state.check_ty_usage(ty);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_unused_import() {
        crate::tests::test_detector("unused_import", 2);
    }
}
