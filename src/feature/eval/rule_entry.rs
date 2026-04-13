use crate::feature::expr::CrateSetExpr;
use super::{CrateSet, PolicyEntry};

pub struct RuleEntry {
    pub from: CrateSet,
    pub internal: Vec<PolicyEntry>,
    pub external: Vec<PolicyEntry>,
    pub metadata: CrateSetExpr,
}
