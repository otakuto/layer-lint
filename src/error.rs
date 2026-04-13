use crate::{CrateName, PolicyKind};
use crate::feature::expr::CrateSetExpr;

pub enum LintError {
    Denied {
        from: CrateName,
        to: CrateName,
        rule_target: CrateSetExpr,
        policy_target: Option<CrateSetExpr>,
    },
    UnusedIgnore {
        from: CrateSetExpr,
        to: CrateSetExpr,
    },
    NoMatchTarget {
        from: CrateSetExpr,
    },
    UnusedAllow {
        from: CrateSetExpr,
        to: CrateSetExpr,
        policy: PolicyKind,
    },
    UndefinedLayer {
        layer: String,
        context: String,
    },
    LayerCycle {
        cycle: Vec<String>,
    },
    UncoveredCrate {
        name: CrateName,
    },
}
