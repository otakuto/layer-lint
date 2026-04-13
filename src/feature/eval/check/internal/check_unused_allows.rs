use std::collections::HashMap;

use crate::{CrateName, LintError, PolicyKind};
use crate::feature::eval::{CrateSet, RuleEvaluator};

/// Check for unused allow entries in rules.
/// An allow ref is unused if no crate matching the rule target has a dependency matching that ref.
/// Deny entries are intentional guards and are NOT checked.
pub fn check_unused_allows(
    evaluator: &RuleEvaluator,
    workspace_deps: &HashMap<CrateName, CrateSet>,
) -> Vec<LintError> {
    let mut errors = Vec::new();

    for rule in &evaluator.rules {
        // Find all source crates that match the rule target.
        let matched_sources: Vec<&CrateName> = workspace_deps
            .keys()
            .filter(|source| rule.from.0.contains(source))
            .collect();

        if matched_sources.is_empty() {
            continue;
        }

        // Collect all actual deps of matched sources.
        let all_deps: Vec<&CrateName> = matched_sources
            .iter()
            .flat_map(|source| {
                workspace_deps
                    .get(*source)
                    .map(|deps| deps.0.iter().collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .collect();

        for policy_entry in rule.internal.iter().chain(&rule.external) {
            // Skip deny and ignore policies
            if policy_entry.policy == PolicyKind::Deny || policy_entry.policy == PolicyKind::Ignore {
                continue;
            }

            for (dep_crate_set, dep_crate_set_expr) in policy_entry.crate_sets.iter().zip(&policy_entry.metadata) {
                let is_used = all_deps.iter().any(|dep| dep_crate_set.0.contains(dep));

                if !is_used {
                    errors.push(LintError::UnusedAllow {
                        from: rule.metadata.clone(),
                        policy: policy_entry.policy,
                        to: dep_crate_set_expr.clone(),
                    });
                }
            }
        }
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
    fn unused_allow_detected() {
        // from: app-server, allow: [serde]
        // app-server は serde に依存していない → UnusedAllow
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("serde"))])],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "serde"]);
        // app-server has no deps
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &[]),
        ]);

        let errors = check_unused_allows(&evaluator, &workspace_deps);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], LintError::UnusedAllow { policy, .. } if *policy == PolicyKind::Allow));
    }

    #[test]
    fn used_allow_not_reported() {
        // from: app-server, allow: [serde]
        // app-server は serde に依存している → no error
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("serde"))])],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "serde"]);
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &["serde"]),
        ]);

        let errors = check_unused_allows(&evaluator, &workspace_deps);
        assert!(errors.is_empty());
    }
}
