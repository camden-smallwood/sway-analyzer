mod discarded_assignments;
mod inline_assembly_usage;
mod missing_logs;
mod msg_amount_in_loop;
mod storage_not_updated;
mod visitor;

pub use self::visitor::*;

use self::{
    discarded_assignments::*, inline_assembly_usage::*, missing_logs::*, msg_amount_in_loop::*,
    storage_not_updated::*,
};

type VisitorConstructor = fn() -> Box<dyn AstVisitor>;
type VisitorEntry = (&'static str, VisitorConstructor);

pub const VISITOR_TYPES: &[VisitorEntry] = &[
    ("discarded_assignments", || Box::new(DiscardedAssignmentsVisitor::default())),
    ("inline_assembly_usage", || Box::new(InlineAssemblyUsageVisitor::default())),
    ("missing_logs", || Box::new(MissingLogsVisitor::default())),
    ("msg_amount_in_loop", || Box::new(MsgAmountInLoopVisitor::default())),
    ("storage_not_updated", || Box::new(StorageNotUpdatedVisitor::default())),
];
