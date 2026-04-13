use std::collections::HashMap;

use crate::{CrateName, LintError};
use crate::feature::eval::{CrateSet, RuleEvaluator};

pub fn check_rules(
    evaluator: &RuleEvaluator,
    workspace_deps: &HashMap<CrateName, CrateSet>,
) -> Vec<LintError> {
    let mut errors = Vec::new();
    for (from, to) in workspace_deps {
        errors.extend(evaluator.check_policy(from, to));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrateName, LintError, PolicyKind};
    use crate::feature::expr::{CrateSetExpr, ConfigExpr, PolicyEntryExpr, RuleEntryExpr};
    use crate::feature::eval::{CrateSet, RuleEvaluator};
    use crate::feature::crate_dependency::{CrateDependency, WorkspaceDependency};
    use std::collections::HashMap;

    fn cn(s: &str) -> CrateName {
        CrateName(s.to_string())
    }

    fn cs(deps: &[&str]) -> CrateSet {
        CrateSet(deps.iter().map(|s| cn(s)).collect())
    }

    fn make_workspace_deps(deps: Vec<(&str, &[&str])>) -> HashMap<CrateName, CrateSet> {
        deps.into_iter().map(|(from, to)| (cn(from), cs(to))).collect()
    }

    fn make_workspace(crate_names: &[&str]) -> WorkspaceDependency {
        let members = crate_names.iter().map(|name| {
            let cn_name = cn(name);
            (cn_name.clone(), CrateDependency { from: cn_name.clone(), to: CrateSet(std::collections::HashSet::new()) })
        }).collect();
        WorkspaceDependency { members, external: Vec::new() }
    }

    fn make_evaluator(rules: Vec<RuleEntryExpr>, crate_names: &[&str]) -> RuleEvaluator {
        let config = ConfigExpr { internal_layers: HashMap::new(), external_layers: HashMap::new(), rules };
        let workspace = make_workspace(crate_names);
        RuleEvaluator::new(config, &workspace)
    }

    fn make_rule(from: CrateSetExpr, policies: Vec<(PolicyKind, Vec<CrateSetExpr>)>) -> RuleEntryExpr {
        RuleEntryExpr {
            from,
            internal: policies.into_iter().map(|(kind, crate_sets)| {
                PolicyEntryExpr { policy: kind, crate_sets }
            }).collect(),
            external: vec![],
        }
    }

    #[test]
    fn check_rules_finds_violations() {
        // deny 違反を検出
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "diesel", "serde"]);
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &["diesel", "serde"]),
        ]);

        let errors = check_rules(&evaluator, &workspace_deps);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], LintError::Denied { from, to, .. } if from == &cn("app-server") && to == &cn("diesel")));
    }

    #[test]
    fn check_rules_no_errors_when_clean() {
        // 違反なしの場合空
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("serde"))])],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "serde"]);
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &["serde"]),
        ]);

        let errors = check_rules(&evaluator, &workspace_deps);
        assert!(errors.is_empty());
    }
}
