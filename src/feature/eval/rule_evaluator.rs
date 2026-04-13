use std::collections::HashMap;

use crate::infra::regex_cache::regex_match;
use crate::{CrateName, LayerName, LintError, PolicyKind};
use crate::feature::expr::{CrateSetExpr, ConfigExpr, RuleEntryExpr};
use super::{CrateSet, PolicyEntry, RuleEntry, SourcePolicy};

pub struct RuleEvaluator {
    pub internal_layers: HashMap<LayerName, CrateSet>,
    pub external_layers: HashMap<LayerName, CrateSet>,
    /// Kept for check_unused_allows and check_unused_ignores iteration.
    pub rules: Vec<RuleEntry>,
    /// Pre-computed per-source policy maps built at construction time.
    pub source_policies: HashMap<CrateName, SourcePolicy>,
}

impl RuleEvaluator {
    pub fn new(config: ConfigExpr, workspace: &crate::feature::crate_dependency::WorkspaceDependency) -> Self {
        let all_crates = workspace.all_crates();
        let member_names: std::collections::HashSet<CrateName> = workspace.members.keys().cloned().collect();
        let member_crates = CrateSet(member_names.clone());
        let external_crates = CrateSet(all_crates.0.difference(&member_names).cloned().collect());
        let internal_layers = resolve_layers(&config.internal_layers, &member_crates);
        let external_layers = resolve_layers(&config.external_layers, &external_crates);
        let rules = resolve_rules(config.rules, &internal_layers, &external_layers, &all_crates);
        let source_policies = resolve_source_policies(&rules, &all_crates, &member_names);
        RuleEvaluator { internal_layers, external_layers, rules, source_policies }
    }

    pub fn check_policy(&self, from: &CrateName, to: &CrateSet) -> Vec<LintError> {
        let policy = match self.source_policies.get(from) {
            Some(p) => p,
            None => return Vec::new(),
        };

        to.0.iter()
            .filter_map(|dep| {
                match policy.resolved.get(dep) {
                    Some((PolicyKind::Ignore, _, _)) => None,
                    Some((PolicyKind::Allow, _, _)) => None,
                    Some((PolicyKind::Deny, rule_from, policy_target)) => Some(LintError::Denied {
                        from: from.clone(),
                        to: dep.clone(),
                        rule_target: rule_from.clone(),
                        policy_target: Some(policy_target.clone()),
                    }),
                    None if policy.default_deny => Some(LintError::Denied {
                        from: from.clone(),
                        to: dep.clone(),
                        rule_target: policy.default_deny_from.as_ref().unwrap().clone(),
                        policy_target: None,
                    }),
                    None => None,
                }
            })
            .collect()
    }
}

fn resolve_source_policies(
    rules: &[RuleEntry],
    all_crates: &CrateSet,
    member_names: &std::collections::HashSet<CrateName>,
) -> HashMap<CrateName, SourcePolicy> {
    let mut result = HashMap::new();

    for source in &all_crates.0 {
        let mut resolved: HashMap<CrateName, (PolicyKind, CrateSetExpr, CrateSetExpr)> = HashMap::new();
        let mut resolved_no_ignore: HashMap<CrateName, PolicyKind> = HashMap::new();
        let mut first_non_ignore: Option<(PolicyKind, CrateSetExpr)> = None;
        let mut has_any_rule = false;

        for rule in rules {
            if !rule.from.0.contains(source) {
                continue;
            }
            has_any_rule = true;

            // internal policies → member crate deps only
            apply_policies(&rule.internal, &rule.metadata, member_names,
                &mut resolved, &mut resolved_no_ignore, &mut first_non_ignore);

            // external policies → external crate deps only
            let external_names: std::collections::HashSet<CrateName> = all_crates.0.iter()
                .filter(|n| !member_names.contains(n))
                .cloned()
                .collect();
            apply_policies(&rule.external, &rule.metadata, &external_names,
                &mut resolved, &mut resolved_no_ignore, &mut first_non_ignore);
        }

        if !has_any_rule {
            continue;
        }

        let default_deny = first_non_ignore.as_ref().is_some_and(|(kind, _)| *kind == PolicyKind::Allow);
        let default_deny_from = if default_deny {
            first_non_ignore.map(|(_, expr)| expr)
        } else {
            None
        };

        result.insert(source.clone(), SourcePolicy {
            resolved,
            resolved_no_ignore,
            default_deny,
            default_deny_from,
        });
    }

    result
}

fn apply_policies(
    policies: &[PolicyEntry],
    rule_metadata: &CrateSetExpr,
    allowed_deps: &std::collections::HashSet<CrateName>,
    resolved: &mut HashMap<CrateName, (PolicyKind, CrateSetExpr, CrateSetExpr)>,
    resolved_no_ignore: &mut HashMap<CrateName, PolicyKind>,
    first_non_ignore: &mut Option<(PolicyKind, CrateSetExpr)>,
) {
    for policy_entry in policies {
        if first_non_ignore.is_none() && policy_entry.policy != PolicyKind::Ignore {
            *first_non_ignore = Some((policy_entry.policy, rule_metadata.clone()));
        }
        for (cs, expr) in policy_entry.crate_sets.iter().zip(&policy_entry.metadata) {
            for crate_name in &cs.0 {
                if !allowed_deps.contains(crate_name) {
                    continue;
                }
                resolved.insert(crate_name.clone(), (policy_entry.policy, rule_metadata.clone(), expr.clone()));
                if policy_entry.policy != PolicyKind::Ignore {
                    resolved_no_ignore.insert(crate_name.clone(), policy_entry.policy);
                }
            }
        }
        // Excluded crates: clear any previous verdict from earlier rules
        for crate_name in &policy_entry.excluded.0 {
            if !allowed_deps.contains(crate_name) {
                continue;
            }
            resolved.remove(crate_name);
            resolved_no_ignore.remove(crate_name);
        }
    }
}

fn resolve_rules(
    rule_exprs: Vec<RuleEntryExpr>,
    internal_layers: &HashMap<LayerName, CrateSet>,
    external_layers: &HashMap<LayerName, CrateSet>,
    all_crates: &CrateSet,
) -> Vec<RuleEntry> {
    rule_exprs.into_iter().map(|rule_expr| {
        let from = resolve_expr(internal_layers, &rule_expr.from, all_crates);
        let metadata = rule_expr.from;
        let internal = resolve_policy_entries(rule_expr.internal, internal_layers, all_crates);
        let external = resolve_policy_entries(rule_expr.external, external_layers, all_crates);
        RuleEntry { from, internal, external, metadata }
    }).collect()
}

fn resolve_policy_entries(
    entries: Vec<crate::feature::expr::PolicyEntryExpr>,
    layers: &HashMap<LayerName, CrateSet>,
    all_crates: &CrateSet,
) -> Vec<PolicyEntry> {
    entries.into_iter().map(|pe| {
        let (crate_sets, excluded, metadata) = resolve_policy_crate_sets(layers, &pe.crate_sets, all_crates);
        PolicyEntry { policy: pe.policy, crate_sets, excluded, metadata }
    }).collect()
}

/// Resolve a single CrateSetExpr to a CrateSet against all known crates.
fn resolve_expr(
    layers: &HashMap<LayerName, CrateSet>,
    expr: &CrateSetExpr,
    all_crates: &CrateSet,
) -> CrateSet {
    CrateSet(all_crates.0.iter()
        .filter(|name| crate_matches(layers, expr, name))
        .cloned()
        .collect())
}

/// Resolve policy crate_sets, handling Exclude.
/// Returns (resolved CrateSets, excluded CrateSet, original CrateSetExprs excluding Exclude entries).
fn resolve_policy_crate_sets(
    layers: &HashMap<LayerName, CrateSet>,
    exprs: &[CrateSetExpr],
    all_crates: &CrateSet,
) -> (Vec<CrateSet>, CrateSet, Vec<CrateSetExpr>) {
    // Collect excluded crate names
    let mut excluded_set = std::collections::HashSet::new();
    for expr in exprs {
        if let CrateSetExpr::Exclude(excludes) = expr {
            for ex in excludes {
                excluded_set.extend(resolve_expr(layers, ex, all_crates).0);
            }
        }
    }

    // Resolve non-Exclude entries and subtract excluded
    let mut crate_sets = Vec::new();
    let mut metadata = Vec::new();
    for expr in exprs {
        if matches!(expr, CrateSetExpr::Exclude(_)) {
            continue;
        }
        let mut resolved = resolve_expr(layers, expr, all_crates);
        resolved.0 = resolved.0.difference(&excluded_set).cloned().collect();
        crate_sets.push(resolved);
        metadata.push(expr.clone());
    }
    (crate_sets, CrateSet(excluded_set), metadata)
}

/// Check if a crate name matches a single CrateSetExpr, using resolved layers.
fn crate_matches(layers: &HashMap<LayerName, CrateSet>, crate_set: &CrateSetExpr, crate_name: &CrateName) -> bool {
    match crate_set {
        CrateSetExpr::Crate(name) => name == crate_name,
        CrateSetExpr::Regex(pattern) => regex_match(&pattern.0, &crate_name.0).is_some(),
        CrateSetExpr::Layer(layer_name) => {
            layers.get(layer_name)
                .map(|resolved| resolved.0.contains(crate_name))
                .unwrap_or(false)
        }
        CrateSetExpr::Exclude(_) => false,
    }
}

/// Resolve all layer definitions into concrete crate name sets.
fn resolve_layers(
    raw_layers: &HashMap<LayerName, Vec<CrateSetExpr>>,
    scope: &CrateSet,
) -> HashMap<LayerName, CrateSet> {
    raw_layers.iter().map(|(name, exprs)| {
        let resolved: std::collections::HashSet<CrateName> = scope.0.iter()
            .filter(|crate_name| raw_list_matches(raw_layers, exprs, crate_name))
            .cloned()
            .collect();
        (name.clone(), CrateSet(resolved))
    }).collect()
}

/// Match using raw (unresolved) layer definitions. Used only during layer resolution.
fn raw_crate_matches(layers: &HashMap<LayerName, Vec<CrateSetExpr>>, crate_set: &CrateSetExpr, crate_name: &CrateName) -> bool {
    match crate_set {
        CrateSetExpr::Crate(name) => name == crate_name,
        CrateSetExpr::Regex(pattern) => regex_match(&pattern.0, &crate_name.0).is_some(),
        CrateSetExpr::Layer(layer_name) => {
            layers.get(layer_name)
                .map(|members| raw_list_matches(layers, members, crate_name))
                .unwrap_or(false)
        }
        CrateSetExpr::Exclude(_) => false,
    }
}

/// Match using raw (unresolved) layer definitions. Used only during layer resolution.
fn raw_list_matches(layers: &HashMap<LayerName, Vec<CrateSetExpr>>, crate_sets: &[CrateSetExpr], crate_name: &CrateName) -> bool {
    let included = crate_sets.iter().any(|r| match r {
        CrateSetExpr::Exclude(_) => false,
        other => raw_crate_matches(layers, other, crate_name),
    });
    if !included {
        return false;
    }
    let excluded = crate_sets.iter().any(|r| match r {
        CrateSetExpr::Exclude(excludes) => {
            excludes.iter().any(|ex| raw_crate_matches(layers, ex, crate_name))
        }
        _ => false,
    });
    !excluded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrateName, LayerName, RegexPattern, PolicyKind};
    use crate::feature::expr::{CrateSetExpr, ConfigExpr, PolicyEntryExpr, RuleEntryExpr};
    use crate::feature::crate_dependency::{CrateDependency, WorkspaceDependency};
    use std::collections::HashMap;

    fn cn(s: &str) -> CrateName {
        CrateName(s.to_string())
    }

    fn cs(deps: &[&str]) -> CrateSet {
        CrateSet(deps.iter().map(|s| cn(s)).collect())
    }

    fn make_workspace(crate_names: &[&str]) -> WorkspaceDependency {
        let members = crate_names.iter().map(|name| {
            let cn_name = cn(name);
            (cn_name.clone(), CrateDependency { from: cn_name.clone(), to: CrateSet(std::collections::HashSet::new()) })
        }).collect();
        WorkspaceDependency { members, external: Vec::new() }
    }

    fn make_config(rules: Vec<RuleEntryExpr>) -> ConfigExpr {
        ConfigExpr {
            internal_layers: HashMap::<LayerName, Vec<CrateSetExpr>>::new(),
            external_layers: HashMap::<LayerName, Vec<CrateSetExpr>>::new(),
            rules,
        }
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
    fn deny_detected() {
        // from: app-server, deny: [diesel]
        // app-server が diesel に依存 → denied
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            LintError::Denied { from, to, .. } => {
                assert_eq!(from, &cn("app-server"));
                assert_eq!(to, &cn("diesel"));
            }
            _ => panic!("expected Denied"),
        }
    }

    #[test]
    fn allow_permits() {
        // from: app-server, allow: [serde]
        // app-server が serde に依存 → ok
        // app-server が diesel に依存 → denied (default deny)
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("serde"))])],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "serde", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let serde_errors = evaluator.check_policy(&cn("app-server"), &cs(&["serde"]));
        assert!(serde_errors.is_empty(), "serde should be allowed");

        let diesel_errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert_eq!(diesel_errors.len(), 1, "diesel should be denied by default");
        assert!(matches!(&diesel_errors[0], LintError::Denied { from, to, .. } if from == &cn("app-server") && to == &cn("diesel")));
    }

    #[test]
    fn ignore_skips() {
        // from: app-server, deny: [diesel], ignore: [diesel]
        // ignore が後に来るので diesel は ignored (no error)
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![
                (PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))]),
                (PolicyKind::Ignore, vec![CrateSetExpr::Crate(cn("diesel"))]),
            ],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert!(errors.is_empty(), "diesel should be ignored");
    }

    #[test]
    fn last_match_wins() {
        // rule1: from app-server, deny: [diesel]
        // rule2: from app-server, allow: [diesel]
        // → diesel は allowed (last match wins)
        let rule1 = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let rule2 = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let config = make_config(vec![rule1, rule2]);
        let workspace = make_workspace(&["app-server", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert!(errors.is_empty(), "diesel should be allowed by last matching rule");
    }

    #[test]
    fn default_deny() {
        // from: app-server, allow: [serde]
        // app-server が diesel に依存 → default denied
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Allow, vec![CrateSetExpr::Crate(cn("serde"))])],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "serde", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], LintError::Denied { from, to, .. } if from == &cn("app-server") && to == &cn("diesel")));
    }

    #[test]
    fn no_rule_no_error() {
        // from: app-server のルールのみ
        // other-crate が何に依存しても → no error
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "other-crate", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let errors = evaluator.check_policy(&cn("other-crate"), &cs(&["diesel"]));
        assert!(errors.is_empty(), "other-crate has no rule, should produce no error");
    }

    #[test]
    fn layer_resolution() {
        // layer "domain" internal = [regex: "^app-entity-(.+)$"]
        // from: layer:domain, deny: [diesel]
        // app-entity-foo が diesel に依存 → denied
        // app-server が diesel に依存 → no error (ルールにマッチしない)
        let mut layers = HashMap::new();
        layers.insert(
            LayerName("domain".to_string()),
            vec![CrateSetExpr::Regex(RegexPattern("^app-entity-(.+)$".to_string()))],
        );
        let rule = make_rule(
            CrateSetExpr::Layer(LayerName("domain".to_string())),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(cn("diesel"))])],
        );
        let config = ConfigExpr { internal_layers: layers, external_layers: HashMap::new(), rules: vec![rule] };
        let workspace = make_workspace(&["app-entity-foo", "app-entity-bar", "app-server", "diesel"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        let entity_errors = evaluator.check_policy(&cn("app-entity-foo"), &cs(&["diesel"]));
        assert_eq!(entity_errors.len(), 1, "app-entity-foo should be denied diesel");

        let server_errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert!(server_errors.is_empty(), "app-server has no rule, should produce no error");
    }

    #[test]
    fn exclude_works() {
        // from: app-server, deny: [regex "^app-(.+)$", exclude: [app-entity-bar]]
        // app-server が app-entity-foo に依存 → denied (app-entity-foo は除外されていない)
        // app-server が app-entity-bar に依存 → no error (app-entity-bar は除外)
        let rule = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![
                CrateSetExpr::Regex(RegexPattern("^app-entity-(.+)$".to_string())),
                CrateSetExpr::Exclude(vec![CrateSetExpr::Crate(cn("app-entity-bar"))]),
            ])],
        );
        let config = make_config(vec![rule]);
        let workspace = make_workspace(&["app-server", "app-entity-foo", "app-entity-bar"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        // app-entity-foo matches regex and is NOT excluded → denied
        let foo_errors = evaluator.check_policy(&cn("app-server"), &cs(&["app-entity-foo"]));
        assert_eq!(foo_errors.len(), 1, "app-entity-foo should be denied");

        // app-entity-bar matches regex but IS excluded → no error
        let bar_errors = evaluator.check_policy(&cn("app-server"), &cs(&["app-entity-bar"]));
        assert!(bar_errors.is_empty(), "app-entity-bar should be excluded from deny");
    }

    #[test]
    fn exclude_clears_prior_deny() {
        // rule1: from app-server, deny: [diesel, serde]
        // rule2: from app-server, deny: [regex "^(.+)$", exclude: [diesel]]
        // → rule2 の exclude で diesel の deny がクリアされる
        // → diesel は denied ではない
        // → serde は rule2 の deny で上書き（excluded ではない）→ denied
        let rule1 = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![
                CrateSetExpr::Crate(cn("diesel")),
                CrateSetExpr::Crate(cn("serde")),
            ])],
        );
        let rule2 = make_rule(
            CrateSetExpr::Crate(cn("app-server")),
            vec![(PolicyKind::Deny, vec![
                CrateSetExpr::Regex(RegexPattern("^(.+)$".to_string())),
                CrateSetExpr::Exclude(vec![CrateSetExpr::Crate(cn("diesel"))]),
            ])],
        );
        let config = make_config(vec![rule1, rule2]);
        let workspace = make_workspace(&["app-server", "diesel", "serde"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        // diesel: rule1 で deny → rule2 の exclude でクリア → no error
        let diesel_errors = evaluator.check_policy(&cn("app-server"), &cs(&["diesel"]));
        assert!(diesel_errors.is_empty(), "diesel should be cleared by exclude");

        // serde: rule1 で deny → rule2 で deny (not excluded) → denied
        let serde_errors = evaluator.check_policy(&cn("app-server"), &cs(&["serde"]));
        assert_eq!(serde_errors.len(), 1, "serde should still be denied");
    }

    #[test]
    fn exclude_clears_default_deny_layer() {
        // default-rule: deny all internal
        // id-rule: deny all external, exclude: [serde, diesel]
        // → serde, diesel は excluded なので denied ではない
        // → uuid は deny されたまま
        let rule1 = make_rule(
            CrateSetExpr::Regex(RegexPattern("^(.+)$".to_string())),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Regex(RegexPattern("^(.+)$".to_string()))])],
        );
        let rule2 = make_rule(
            CrateSetExpr::Crate(cn("app-id-foo")),
            vec![(PolicyKind::Deny, vec![
                CrateSetExpr::Regex(RegexPattern("^(.+)$".to_string())),
                CrateSetExpr::Exclude(vec![
                    CrateSetExpr::Crate(cn("serde")),
                    CrateSetExpr::Crate(cn("diesel")),
                ]),
            ])],
        );
        let config = make_config(vec![rule1, rule2]);
        let workspace = make_workspace(&["app-id-foo", "serde", "diesel", "uuid"]);
        let evaluator = RuleEvaluator::new(config, &workspace);

        // serde: excluded → no error
        let serde_errors = evaluator.check_policy(&cn("app-id-foo"), &cs(&["serde"]));
        assert!(serde_errors.is_empty(), "serde should be cleared by exclude");

        // diesel: excluded → no error
        let diesel_errors = evaluator.check_policy(&cn("app-id-foo"), &cs(&["diesel"]));
        assert!(diesel_errors.is_empty(), "diesel should be cleared by exclude");

        // uuid: not excluded → denied
        let uuid_errors = evaluator.check_policy(&cn("app-id-foo"), &cs(&["uuid"]));
        assert_eq!(uuid_errors.len(), 1, "uuid should still be denied");
    }
}
