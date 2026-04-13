use crate::PolicyKind;
use super::CrateSetExpr;

pub struct PolicyEntryExpr {
    pub policy: PolicyKind,
    pub crate_sets: Vec<CrateSetExpr>,
}
