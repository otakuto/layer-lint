use std::collections::HashSet;

use crate::{LintError, LayerName};
use crate::feature::expr::{CrateSetExpr, ConfigExpr};

pub fn check_layer_cycles(config: &ConfigExpr) -> Vec<LintError> {
    let mut errors = Vec::new();
    let mut globally_visited = HashSet::new();

    let all_layer_names: Vec<&LayerName> = config.internal_layers.keys()
        .chain(config.external_layers.keys())
        .collect();

    for layer_name in all_layer_names {
        if globally_visited.contains(&layer_name.0) {
            continue;
        }
        let mut path = Vec::new();
        let mut path_set = HashSet::new();
        detect_cycle(
            config,
            &layer_name.0,
            &mut path,
            &mut path_set,
            &mut globally_visited,
            &mut errors,
        );
    }

    errors
}

fn detect_cycle(
    config: &ConfigExpr,
    layer_name: &str,
    path: &mut Vec<String>,
    path_set: &mut HashSet<String>,
    globally_visited: &mut HashSet<String>,
    errors: &mut Vec<LintError>,
) {
    if path_set.contains(layer_name) {
        let cycle_start = path.iter().position(|n| n == layer_name).unwrap();
        let mut cycle: Vec<String> = path[cycle_start..].to_vec();
        cycle.push(layer_name.to_string());
        errors.push(LintError::LayerCycle { cycle });
        return;
    }

    if globally_visited.contains(layer_name) {
        return;
    }

    path.push(layer_name.to_string());
    path_set.insert(layer_name.to_string());

    let key = LayerName(layer_name.to_string());
    let layer_def_opt = config.internal_layers.get(&key)
        .or_else(|| config.external_layers.get(&key));
    if let Some(layer_def) = layer_def_opt {
        let mut child_layers = Vec::new();
        for member in layer_def.iter() {
            collect_layer_names(member, &mut child_layers);
        }
        for child in child_layers {
            detect_cycle(config, &child, path, path_set, globally_visited, errors);
        }
    }

    path.pop();
    path_set.remove(layer_name);
    globally_visited.insert(layer_name.to_string());
}

fn collect_layer_names(crate_set: &CrateSetExpr, layers: &mut Vec<String>) {
    match crate_set {
        CrateSetExpr::Layer(name) => layers.push(name.0.clone()),
        CrateSetExpr::Exclude(excludes) => {
            for ex in excludes {
                collect_layer_names(ex, layers);
            }
        }
        CrateSetExpr::Crate(_) | CrateSetExpr::Regex(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayerName, LintError};
    use crate::feature::expr::{CrateSetExpr, ConfigExpr};

    fn make_config(layers: Vec<(&str, Vec<CrateSetExpr>)>) -> ConfigExpr {
        let internal_layers = layers
            .into_iter()
            .map(|(name, exprs)| (LayerName(name.to_string()), exprs))
            .collect();
        ConfigExpr { internal_layers, external_layers: std::collections::HashMap::new(), rules: Vec::new() }
    }

    fn layer_ref(name: &str) -> CrateSetExpr {
        CrateSetExpr::Layer(LayerName(name.to_string()))
    }

    #[test]
    fn cycle_detected() {
        // a → b → a (循環参照)
        let config = make_config(vec![
            ("a", vec![layer_ref("b")]),
            ("b", vec![layer_ref("a")]),
        ]);

        let errors = check_layer_cycles(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, LintError::LayerCycle { .. })));
    }

    #[test]
    fn no_cycle() {
        // a → b → c (循環なし)
        let config = make_config(vec![
            ("a", vec![layer_ref("b")]),
            ("b", vec![layer_ref("c")]),
            ("c", vec![]),
        ]);

        let errors = check_layer_cycles(&config);
        assert!(errors.is_empty());
    }
}
