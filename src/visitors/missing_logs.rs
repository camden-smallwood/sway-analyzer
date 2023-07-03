use super::{AstVisitor, BlockContext, FnContext, ModuleContext, StatementContext, UseContext, ExprContext};
use crate::{error::Error, project::Project, utils};
use std::{collections::HashMap, path::PathBuf};
use sway_ast::{UseTree, Expr};
use sway_types::{Span, Spanned};

#[derive(Default)]
pub struct MissingLogsVisitor {
    module_states: HashMap<PathBuf, ModuleState>,
}

#[derive(Default)]
struct ModuleState {
    log_names: Vec<String>,
    fn_states: HashMap<Span, FnState>,
}

#[derive(Default)]
struct FnState {
    block_states: HashMap<Span, BlockState>,
}

#[derive(Default)]
struct BlockState {
    written: Vec<(Span, Span)>,
    logged: Vec<Span>,
}

impl AstVisitor for MissingLogsVisitor {
    fn visit_module(&mut self, context: &ModuleContext, _project: &mut Project) -> Result<(), Error> {
        // Create the module state
        if !self.module_states.contains_key(context.path) {
            self.module_states.insert(context.path.into(), ModuleState::default());
        }

        Ok(())
    }

    fn visit_use(&mut self, context: &UseContext, _project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path).unwrap();

        // Destructure the use tree
        let UseTree::Path { prefix, suffix, .. } = &context.item_use.tree else { return Ok(()) };
        let "std" = prefix.as_str() else { return Ok(()) };
        let UseTree::Path { prefix, suffix, .. } = suffix.as_ref() else { return Ok(()) };
        let "logging" = prefix.as_str() else { return Ok(()) };

        match suffix.as_ref() {
            UseTree::Name { name } if name.as_str() == "log" => {
                module_state.log_names.push(name.as_str().to_string());
            }

            UseTree::Rename { name, alias, .. } if name.as_str() == "log" => {
                module_state.log_names.push(alias.as_str().to_string());
            }

            _ => {}
        }

        Ok(())
    }

    fn visit_fn(&mut self, context: &FnContext, _project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path).unwrap();

        // Create the function state
        let fn_signature = context.item_fn.fn_signature.span();
        
        if !module_state.fn_states.contains_key(&fn_signature) {
            module_state.fn_states.insert(fn_signature, FnState::default());
        }
        
        Ok(())
    }

    fn visit_block(&mut self, context: &BlockContext, _project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path).unwrap();

        // Get the function state
        let fn_signature = context.item_fn.fn_signature.span();
        let fn_state = module_state.fn_states.get_mut(&fn_signature).unwrap();

        // Create the block state
        let block_span = context.block.span();

        if !fn_state.block_states.contains_key(&block_span) {
            fn_state.block_states.insert(block_span, BlockState::default());
        }
        
        Ok(())
    }

    fn leave_block(&mut self, context: &BlockContext, project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path).unwrap();

        // Get the function state
        let fn_signature = context.item_fn.fn_signature.span();
        let fn_state = module_state.fn_states.get_mut(&fn_signature).unwrap();

        // Get the block state
        let block_span = context.block.span();
        let block_state = fn_state.block_states.get_mut(&block_span).unwrap();

        // Check each written storage variable to see if it has been logged
        for (storage_span, var_span) in block_state.written.iter() {
            if block_state.logged.iter().find(|logged| logged.as_str() == var_span.as_str()).is_none() {
                project.report.borrow_mut().add_entry(
                    context.path,
                    project.span_to_line(context.path, storage_span)?,
                    format!("The `storage.{}` value is written without being logged.", storage_span.as_str()),
                );
            }
        }

        Ok(())
    }

    fn visit_statement(&mut self, context: &StatementContext, _project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path).unwrap();

        // Get the function state
        let fn_signature = context.item_fn.fn_signature.span();
        let fn_state = module_state.fn_states.get_mut(&fn_signature).unwrap();

        // Get the block state
        let block_span = context.blocks.last().unwrap();
        let block_state = fn_state.block_states.get_mut(block_span).unwrap();

        // Check for storage writes and add them to the block state
        if let Some((storage_name, var_name)) = utils::statement_to_storage_write_idents(context.statement) {
            block_state.written.push((storage_name.span(), var_name.span()));
        }

        Ok(())
    }

    fn visit_expr(&mut self, context: &ExprContext, _project: &mut Project) -> Result<(), Error> {
        // Get the module state
        let module_state = self.module_states.get_mut(context.path.into()).unwrap();

        // Get the function state
        let fn_signature = context.item_fn.fn_signature.span();
        let fn_state = module_state.fn_states.get_mut(&fn_signature).unwrap();

        // Get the block state
        let block_span = context.blocks.last().unwrap();
        let block_state = fn_state.block_states.get_mut(block_span).unwrap();

        // Destructure the expression into a function application
        let Expr::FuncApp { func, args } = context.expr else { return Ok(()) };
        let Expr::Path(path) = func.as_ref() else { return Ok(()) };

        let mut log_args = vec![];

        for arg in args.inner.value_separator_pairs.iter() {
            log_args.push(&arg.0);
        }

        if let Some(arg) = args.inner.final_value_opt.as_ref() {
            log_args.push(arg.as_ref());
        }

        if log_args.len() != 1 {
            return Ok(());
        }

        let logged_span = log_args.last().unwrap().span();
        
        // Check for calls to the imported `log` function
        if path.suffix.is_empty() {
            for log_name in module_state.log_names.iter() {
                if path.prefix.name.as_str() == log_name {
                    // Add the `log` span to the block state
                    block_state.logged.push(logged_span);
                    break;
                }
            }
        }
        // Check for calls to the `std::logging::log` function
        else if path.suffix.len() == 2 {
            let "std" = path.prefix.name.as_str() else { return Ok(()) };
            let "logging" = path.suffix[0].1.name.as_str() else { return Ok(()) };
            let "log" = path.suffix[1].1.name.as_str() else { return Ok(()) };
            block_state.logged.push(logged_span);
        }

        Ok(())
    }
}
