use std::collections::HashMap;

use crate::{CrateName, PolicyKind};
use crate::feature::expr::CrateSetExpr;

pub struct SourcePolicy {
    /// dep crate → (final PolicyKind, rule metadata for error messages, policy entry crate_set_expr) — last-match-wins
    pub resolved: HashMap<CrateName, (PolicyKind, CrateSetExpr, CrateSetExpr)>,
    /// dep crate → PolicyKind without Ignore entries — used by unused-ignore check
    pub resolved_no_ignore: HashMap<CrateName, PolicyKind>,
    /// true if first non-ignore policy is Allow (unmatched deps are denied by default)
    pub default_deny: bool,
    /// metadata: the first matching rule's from_expr, used in default-deny error messages
    pub default_deny_from: Option<CrateSetExpr>,
}
