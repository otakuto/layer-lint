use std::collections::HashSet;

use crate::LintError;
use crate::feature::expr::{CrateSetExpr, ConfigExpr};

pub fn check_undefined_layers(config: &ConfigExpr) -> Vec<LintError> {
    let all_layers: HashSet<&str> = config.internal_layers.keys()
        .chain(config.external_layers.keys())
        .map(|k| k.0.as_str())
        .collect();
    let mut errors = Vec::new();

    for (layer_name, layer_def) in config.internal_layers.iter().chain(config.external_layers.iter()) {
        let context = format!("layer '{}'", layer_name.0);
        for member in layer_def.iter() {
            check_crate_set(&all_layers, member, &mut errors, &context);
        }
    }

    for rule in &config.rules {
        check_crate_set(&all_layers, &rule.from, &mut errors, "rule target");
        for policy_entry in rule.internal.iter().chain(&rule.external) {
            for member in &policy_entry.crate_sets {
                check_crate_set(&all_layers, member, &mut errors, "rule policy");
            }
        }
    }

    errors
}

fn check_crate_set(
    defined: &HashSet<&str>,
    cs: &CrateSetExpr,
    errors: &mut Vec<LintError>,
    context: &str,
) {
    match cs {
        CrateSetExpr::Layer(name) => {
            if !defined.contains(name.0.as_str()) {
                errors.push(LintError::UndefinedLayer {
                    layer: name.0.clone(),
                    context: context.to_string(),
                });
            }
        }
        CrateSetExpr::Exclude(excludes) => {
            for ex in excludes {
                check_crate_set(defined, ex, errors, context);
            }
        }
        CrateSetExpr::Regex(_) | CrateSetExpr::Crate(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrateName, LayerName, LintError, PolicyKind};
    use crate::feature::expr::{CrateSetExpr, ConfigExpr, PolicyEntryExpr, RuleEntryExpr};

    fn make_config_with_layers(
        layers: Vec<(&str, Vec<CrateSetExpr>)>,
        rules: Vec<RuleEntryExpr>,
    ) -> ConfigExpr {
        let internal_layers = layers
            .into_iter()
            .map(|(name, exprs)| (LayerName(name.to_string()), exprs))
            .collect();
        ConfigExpr { internal_layers, external_layers: std::collections::HashMap::new(), rules }
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
    fn undefined_layer_detected() {
        // rule の from に未定義レイヤーを参照 → UndefinedLayer エラー
        let rule = make_rule(
            CrateSetExpr::Layer(LayerName("nonexistent".to_string())),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(CrateName("diesel".to_string()))])],
        );
        let config = make_config_with_layers(vec![], vec![rule]);

        let errors = check_undefined_layers(&config);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            LintError::UndefinedLayer { layer, .. } => {
                assert_eq!(layer, "nonexistent");
            }
            _ => panic!("expected UndefinedLayer"),
        }
    }

    #[test]
    fn defined_layer_no_error() {
        // rule の from に定義済みレイヤーを参照 → エラーなし
        let rule = make_rule(
            CrateSetExpr::Layer(LayerName("domain".to_string())),
            vec![(PolicyKind::Deny, vec![CrateSetExpr::Crate(CrateName("diesel".to_string()))])],
        );
        let config = make_config_with_layers(
            vec![("domain", vec![])],
            vec![rule],
        );

        let errors = check_undefined_layers(&config);
        assert!(errors.is_empty());
    }
}
