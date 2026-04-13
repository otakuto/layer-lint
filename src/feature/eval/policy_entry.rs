use crate::PolicyKind;
use crate::feature::expr::CrateSetExpr;
use super::CrateSet;

pub struct PolicyEntry {
    pub policy: PolicyKind,
    pub crate_sets: Vec<CrateSet>,
    pub excluded: CrateSet,
    pub metadata: Vec<CrateSetExpr>,
}
