use std::collections::HashMap;

use crate::{CrateName, LintError, PolicyKind};
use crate::feature::eval::{CrateSet, RuleEvaluator};

/// Check for unused ignore entries.
/// An ignore ref is unused if:
/// 1. The target matches no workspace crate, or
/// 2. No matched source crate actually depends on a crate matching the ref, or
/// 3. The dep would not be denied even without the ignore (already allowed by rules)
pub fn check_unused_ignores(
    evaluator: &RuleEvaluator,
    workspace_deps: &HashMap<CrateName, CrateSet>,
) -> Vec<LintError> {
    let mut errors = Vec::new();

    for rule in &evaluator.rules {
        let matched_sources: Vec<&CrateName> = workspace_deps
            .keys()
            .filter(|source| rule.from.0.contains(source))
            .collect();

        if matched_sources.is_empty() {
            let has_ignore = rule.internal.iter().chain(&rule.external).any(|p| p.policy == PolicyKind::Ignore);
            if has_ignore {
                errors.push(LintError::NoMatchTarget {
                    from: rule.metadata.clone(),
                });
            }
            continue;
        }

        for policy_entry in rule.internal.iter().chain(&rule.external) {
            if policy_entry.policy != PolicyKind::Ignore {
                continue;
            }

            for (crate_set, crate_set_expr) in policy_entry.crate_sets.iter().zip(&policy_entry.metadata) {
                let is_used = matched_sources.iter().any(|source| {
                    let policy = match evaluator.source_policies.get(*source) {
                        Some(p) => p,
                        None => return false,
                    };
                    workspace_deps.get(*source).is_some_and(|deps| {
                        deps.0.iter().any(|dep| {
                            if !crate_set.0.contains(dep) {
                                return false;
                            }
                            // Would this dep be denied without the ignore?
                            match policy.resolved_no_ignore.get(dep) {
                                Some(PolicyKind::Deny) => true,
                                None if policy.default_deny => true,
                                _ => false,
                            }
                        })
                    })
                });

                if !is_used {
                    errors.push(LintError::UnusedIgnore {
                        from: rule.metadata.clone(),
                        to: crate_set_expr.clone(),
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
    fn unused_ignore_detected() {
        // from: app-server, deny: [serde], ignore: [uuid]
        // app-server は uuid に依存しているが、deny されていない → UnusedIgnore
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![
                (PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("serde"))]),
                (PolicyKind::Ignore, vec![CrateSetExpr::Crate(cn("uuid"))]),
            ],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "serde", "uuid"]);
        // app-server depends on uuid, but uuid is not denied (deny only covers serde)
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &["uuid"]),
        ]);

        let errors = check_unused_ignores(&evaluator, &workspace_deps);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], LintError::UnusedIgnore { .. }));
    }

    #[test]
    fn used_ignore_not_reported() {
        // from: app-server, deny: [diesel], ignore: [diesel]
        // ignore が後に来るので diesel は ignored — ignore は必要
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![
                (PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))]),
                (PolicyKind::Ignore, vec![CrateSetExpr::Crate(cn("diesel"))]),
            ],
        );
        let evaluator = make_evaluator(vec![rule], &["app-server", "diesel"]);
        let workspace_deps = make_workspace_deps(vec![
            ("app-server", &["diesel"]),
        ]);

        let errors = check_unused_ignores(&evaluator, &workspace_deps);
        assert!(errors.is_empty());
    }
}
