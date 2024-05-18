use crate::project::Project;
use std::{cell::RefCell, rc::Rc};
use sway_ast::{
    brackets::SquareBrackets, keywords::{CloseAngleBracketToken, Keyword, OpenAngleBracketToken, StrToken, Token}, ty::{TyArrayDescriptor, TyTupleDescriptor}, AngleBrackets, CommaToken, DoubleColonToken, Expr, ExprArrayDescriptor, ExprTupleDescriptor, FnSignature, GenericArgs, ItemUse, Literal, MatchBranchKind, Parens, PathExpr, PathExprSegment, PathType, PathTypeSegment, Punctuated, Ty
};
use sway_types::{BaseIdent, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum AstVariableKind {
    Constant,
    Storage,
    Configurable,
    Parameter,
    Local,
}

#[derive(Clone, Debug)]
pub struct AstVariable {
    pub kind: AstVariableKind,
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Default)]
pub struct AstScope {
    pub parent: Option<Rc<RefCell<AstScope>>>,
    pub uses: Vec<ItemUse>,
    pub variables: Vec<Rc<RefCell<AstVariable>>>,
    pub functions: Vec<Rc<RefCell<FnSignature>>>,
}

#[inline]
fn empty_tuple_ty() -> Ty {
    Ty::Tuple(Parens {
        inner: TyTupleDescriptor::Nil,
        span: Span::dummy(),
    })
}

impl AstScope {
    fn dump_uses(&self) {
        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().dump_uses();
        }
        println!("{:#?}", self.uses);
    }

    pub fn get_variable(&self, name: &str, is_storage: bool) -> Option<Rc<RefCell<AstVariable>>> {
        for variable in self.variables.iter().rev() {
            if (variable.borrow().kind == AstVariableKind::Storage) != is_storage {
                continue;
            }

            if variable.borrow().name == name {
                return Some(variable.clone());
            }
        }

        if let Some(parent) = self.parent.as_ref() {
            if let Some(variable) = parent.borrow().get_variable(name, is_storage) {
                return Some(variable.clone());
            }
        }

        None
    }

    pub fn get_full_ty(&self, project: &mut Project, ty: &Ty) -> Ty {
        if project.resolver.borrow().resolve_ty(ty).is_some() {
            return ty.clone();
        }

        match ty {
            Ty::Path(path_type) => {
                //
                // TODO:
                // Turn relative path into full path, i.e: `StorageKey<Option<T>>` => `core::storage::StorageKey<std::option::Option<T>>`
                // We should check the `core::prelude` and `std::prelude` modules first before checking the `use` statements in scope.
                //

                self.dump_uses();
                todo!("{path_type:#?}")
            }
            
            Ty::Tuple(tuple) => Ty::Tuple(Parens {
                inner: match &tuple.inner {
                    TyTupleDescriptor::Nil => TyTupleDescriptor::Nil,
                    TyTupleDescriptor::Cons { head, comma_token, tail } => TyTupleDescriptor::Cons {
                        head: Box::new(self.get_full_ty(project, head)),
                        comma_token: comma_token.clone(),
                        tail: Punctuated {
                            value_separator_pairs: tail.value_separator_pairs.iter()
                                .map(|(ty, comma)| (self.get_full_ty(project, ty), comma.clone()))
                                .collect(),
                            final_value_opt: tail.final_value_opt.as_ref()
                                .map(|ty| Box::new(self.get_full_ty(project, ty))),
                        },
                    },
                },
                span: tuple.span.clone(),
            }),

            Ty::Array(array) => Ty::Array(SquareBrackets {
                inner: TyArrayDescriptor {
                    ty: Box::new(self.get_full_ty(project, &array.inner.ty)),
                    semicolon_token: array.inner.semicolon_token.clone(),
                    length: array.inner.length.clone(),
                },
                span: array.span.clone(),
            }),
            
            Ty::Ptr { ptr_token, ty } => Ty::Ptr {
                ptr_token: ptr_token.clone(),
                ty: SquareBrackets {
                    inner: Box::new(self.get_full_ty(project, &ty.inner)),
                    span: ty.span.clone(),
                },
            },

            Ty::Slice { slice_token, ty } => Ty::Slice {
                slice_token: slice_token.clone(),
                ty: SquareBrackets {
                    inner: Box::new(self.get_full_ty(project, &ty.inner)),
                    span: ty.span.clone(),
                },
            },

            Ty::Ref { ampersand_token, mut_token, ty } => Ty::Ref {
                ampersand_token: ampersand_token.clone(),
                mut_token: mut_token.clone(),
                ty: Box::new(self.get_full_ty(project, ty)),
            },

            Ty::StringSlice(_) |
            Ty::StringArray { .. } |
            Ty::Infer { .. } |
            Ty::Never { .. } => ty.clone(),
        }
    }

    pub fn get_fn_signature(
        &self,
        project: &mut Project,
        fn_name: &PathExprSegment,
        args: &Parens<Punctuated<Expr, CommaToken>>,
    ) -> Option<&FnSignature> {
        //
        // TODO:
        //
        // We need to find the `fn` we are looking for.
        // We need to ensure the argument types of the `fn` match the types of the supplied `args`.
        //
        // If the `fn` is not defined in the current module, we need to find a `use` statement that imports a valid `fn`:
        // 1. Check `prelude` module of the `core` library
        // 2. Check `prelude` module of the `std` library
        // 3. Check all explicit `use` statements
        //
        // Once we find the `fn`, return the signature of the `fn`
        //
        
        todo!()
    }

    pub fn get_impl_fn_signature(
        &self,
        project: &mut Project,
        ty: &Ty,
        fn_name: &PathExprSegment,
        args: &Parens<Punctuated<Expr, CommaToken>>,
    ) -> Option<&FnSignature> {
        //
        // TODO:
        //
        // We need to find a valid `impl` that contains the `fn` we are looking for.
        // We need to ensure the argument types of the `fn` match the types of the supplied `args`.
        //
        // If the `impl` is not defined in the current module, we need to find a `use` statement that imports a valid `impl` containing the `fn`:
        // 1. Check `prelude` module of the `core` library
        // 2. Check `prelude` module of the `std` library
        // 3. Check all explicit `use` statements
        //
        // Once we find the `impl` containing the `fn`, return the signature of the `fn`
        //

        self.dump_uses();
        
        todo!("{:#?}", (ty, fn_name, args))
    }

    pub fn get_expr_ty(&self, expr: &Expr, project: &mut Project) -> Ty {
        match expr {
            Expr::Error(_, _) => todo!("{expr:#?}"),

            Expr::Path(path) => {
                // Check if the path is a single identifier and look it up as a variable
                if path.root_opt.is_none() && path.suffix.is_empty() {
                    if let Some(variable) = self.get_variable(path.prefix.name.as_str(), false) {
                        return variable.borrow().ty.clone();
                    }
                }

                todo!("{expr:#?}")
            }

            Expr::Literal(literal) => match literal {
                Literal::String(_) => Ty::StringSlice(StrToken::new(Span::dummy())),
                Literal::Char(_) => todo!("char does not have a type name in sway yet"),
                Literal::Int(_) => Ty::Path(PathType {
                    root_opt: None,
                    prefix: PathTypeSegment {
                        name: BaseIdent::new_no_span("u64".into()),
                        generics_opt: None,
                    },
                    suffix: vec![],
                }),
                Literal::Bool(_) => Ty::Path(PathType {
                    root_opt: None,
                    prefix: PathTypeSegment {
                        name: BaseIdent::new_no_span("bool".into()),
                        generics_opt: None,
                    },
                    suffix: vec![],
                }),
            }

            Expr::AbiCast { args, .. } => Ty::Path(args.inner.name.clone()),

            Expr::Struct { path, fields } => {
                //
                // TODO:
                // 1. Resolve type using both `path` and `fields`
                // 2. Resolve and return full type path (i.e: StorageKey<T> => std::storage::storage_key::StorageKey<T>)
                //

                Ty::Path(PathType {
                    root_opt: path.root_opt.clone(),
                    prefix: PathTypeSegment {
                        name: path.prefix.name.clone(),
                        generics_opt: path.prefix.generics_opt.clone().map(|(t, g)| (Some(t), g)),
                    },
                    suffix: vec![],
                })
            }

            Expr::Tuple(tuple) => {
                match &tuple.inner {
                    ExprTupleDescriptor::Nil => empty_tuple_ty(),

                    ExprTupleDescriptor::Cons { head, tail, .. } => {
                        let mut value_separator_pairs = vec![];
    
                        for expr in tail {
                            value_separator_pairs.push((self.get_expr_ty(expr, project), CommaToken::new(Span::dummy())));
                        }
    
                        let final_value_opt = value_separator_pairs.pop().map(|x| Box::new(x.0));
    
                        Ty::Tuple(Parens {
                            inner: TyTupleDescriptor::Cons {
                                head: Box::new(self.get_expr_ty(head, project)),
                                comma_token: CommaToken::new(Span::dummy()),
                                tail: Punctuated {
                                    value_separator_pairs,
                                    final_value_opt,
                                },
                            },
                            span: Span::dummy(),
                        })
                    }
                }
            }

            Expr::Parens(parens) => self.get_expr_ty(parens.inner.as_ref(), project),

            Expr::Block(block) => {
                match block.inner.final_expr_opt.as_ref() {
                    Some(expr) => self.get_expr_ty(expr, project),
                    None => empty_tuple_ty()
                }
            }

            Expr::Array(array) => {
                match &array.inner {
                    ExprArrayDescriptor::Sequence(sequence) => {
                        if let Some((expr, _)) = sequence.value_separator_pairs.first() {
                            self.get_expr_ty(expr, project)
                        } else if let Some(expr) = sequence.final_value_opt.as_ref() {
                            self.get_expr_ty(expr, project)
                        } else {
                            empty_tuple_ty()
                        }
                    }

                    ExprArrayDescriptor::Repeat { value, .. } => {
                        self.get_expr_ty(value, project)
                    }
                }
            }

            Expr::Asm(_) => {
                //
                // TODO: Get the type of the return value from the asm block if any
                //

                empty_tuple_ty()
            }

            Expr::Return { .. } => empty_tuple_ty(),

            Expr::If(if_expr) => {
                if let Some(expr) = if_expr.then_block.inner.final_expr_opt.as_ref() {
                    return self.get_expr_ty(expr, project);
                }

                empty_tuple_ty()
            }

            Expr::Match { branches, .. } => {
                if let Some(branch) = branches.inner.first() {
                    match &branch.kind {
                        MatchBranchKind::Block { block, .. } => {
                            if let Some(expr) = block.inner.final_expr_opt.as_ref() {
                                return self.get_expr_ty(expr, project);
                            }
            
                            return empty_tuple_ty();
                        }

                        MatchBranchKind::Expr { expr, .. } => {
                            return self.get_expr_ty(expr, project);
                        }
                    }
                }

                empty_tuple_ty()
            }

            Expr::While { .. } | Expr::For { .. } => empty_tuple_ty(),

            Expr::FuncApp { func, args } => todo!("{expr:#?}"),

            Expr::Index { target, .. } => {
                let target_type = self.get_expr_ty(target, project);

                let Ty::Array(target_type) = target_type else {
                    panic!("Expected array type, got: {target_type:#?}");
                };

                target_type.inner.ty.as_ref().clone()
            }

            Expr::MethodCall { target, path_seg, args, .. } => {
                let target_type = self.get_expr_ty(target, project);
                let fn_signature = self.get_impl_fn_signature(project, &target_type, path_seg, args).unwrap();
                
                let ty = fn_signature.return_type_opt.as_ref()
                    .map(|(_, ty)| ty.clone())
                    .unwrap_or_else(empty_tuple_ty);

                self.get_full_ty(project, &ty)
            }

            Expr::FieldProjection { target, name, .. } => {
                // Check if the field projection refers to a storage field and return a `core::storage::StorageKey<T>` type
                if let Expr::Path(PathExpr { root_opt, prefix, suffix, .. }) = target.as_ref() {
                    if root_opt.is_none() && prefix.name.as_str() == "storage" && suffix.is_empty() {
                        let variable = self.get_variable(name.as_str(), true).unwrap();
                        let ty = self.get_full_ty(project, &variable.borrow().ty);

                        return Ty::Path(PathType {
                            root_opt: None,
                            prefix: PathTypeSegment {
                                name: BaseIdent::new_no_span("core".into()),
                                generics_opt: None,
                            },
                            suffix: vec![
                                (DoubleColonToken::new(Span::dummy()), PathTypeSegment {
                                    name: BaseIdent::new_no_span("storage".into()),
                                    generics_opt: None,
                                }),
                                (DoubleColonToken::new(Span::dummy()), PathTypeSegment {
                                    name: BaseIdent::new_no_span("StorageKey".into()),
                                    generics_opt: Some((None, GenericArgs {
                                        parameters: AngleBrackets {
                                            open_angle_bracket_token: OpenAngleBracketToken::new(Span::dummy()),
                                            inner: Punctuated {
                                                value_separator_pairs: vec![],
                                                final_value_opt: Some(Box::new(ty)),
                                            },
                                            close_angle_bracket_token: CloseAngleBracketToken::new(Span::dummy()),
                                        },
                                    })),
                                }),
                            ],
                        });
                    }
                }

                let target_type = self.get_expr_ty(target, project);
                
                let resolver = project.resolver.borrow();
                let resolved = resolver.resolve_ty(&target_type);
                
                let Some(sway_ast::ItemKind::Struct(item_struct)) = resolved else {
                    panic!("Expected struct, found: {resolved:#?}")
                };

                let mut fields = vec![];

                for field in &item_struct.fields.inner {
                    fields.push(field);
                }

                let Some(field) = fields.iter().find(|f| f.value.name == *name) else {
                    todo!("{expr:#?}")
                };

                field.value.ty.clone()
            }

            Expr::TupleFieldProjection { target, field, .. } => {
                let target_type = self.get_expr_ty(target, project);

                let Ty::Tuple(target_type) = target_type else {
                    panic!("Expected tuple type, got: {target_type:#?}");
                };

                match &target_type.inner {
                    TyTupleDescriptor::Nil => panic!("Field access on empty tuple: {expr:#?}"),
                    
                    TyTupleDescriptor::Cons { head, tail, .. } => {
                        let index: usize = field.try_into().unwrap();
                        
                        if index == 0 {
                            return head.as_ref().clone();
                        }

                        let mut remaining = vec![];

                        for ty in tail {
                            remaining.push(ty);
                        }

                        remaining[index - 1].clone()
                    }
                }
            }

            Expr::Ref { expr, .. } => self.get_expr_ty(expr, project),
            Expr::Deref { expr, .. } => self.get_expr_ty(expr, project),
            
            Expr::Not { expr, .. } => self.get_expr_ty(expr, project),

            Expr::Mul { lhs, .. } |
            Expr::Div { lhs, .. } |
            Expr::Pow { lhs, .. } |
            Expr::Modulo { lhs, .. } |
            Expr::Add { lhs, .. } |
            Expr::Sub { lhs, .. } |
            Expr::Shl { lhs, .. } |
            Expr::Shr { lhs, .. } |
            Expr::BitAnd { lhs, .. } |
            Expr::BitXor { lhs, .. } |
            Expr::BitOr { lhs, .. } => {
                self.get_expr_ty(lhs, project)
            }

            Expr::Equal { .. } |
            Expr::NotEqual { .. } |
            Expr::LessThan { .. } |
            Expr::GreaterThan { .. } |
            Expr::LessThanEq { .. } |
            Expr::GreaterThanEq { .. } |
            Expr::LogicalAnd { .. } |
            Expr::LogicalOr { .. } => {
                Ty::Path(PathType {
                    root_opt: None,
                    prefix: PathTypeSegment {
                        name: BaseIdent::new_no_span("bool".into()),
                        generics_opt: None,
                    },
                    suffix: vec![],
                })
            }

            Expr::Reassignment { .. } => empty_tuple_ty(),

            Expr::Break { .. } | Expr::Continue { .. } => empty_tuple_ty(),
        }
    }
}
