use std::collections::HashMap;

use crate::CrateName;
use crate::LintError;
use crate::feature::eval::{CrateSet, RuleEvaluator};

pub fn check_uncovered_crates(
    evaluator: &RuleEvaluator,
    workspace_deps: &HashMap<CrateName, CrateSet>,
) -> Vec<LintError> {
    let mut uncovered: Vec<&CrateName> = workspace_deps.keys()
        .filter(|name| !evaluator.source_policies.contains_key(name))
        .collect();
    uncovered.sort_by(|a, b| a.0.cmp(&b.0));

    uncovered.into_iter().map(|name| {
        LintError::UncoveredCrate {
            name: name.clone(),
        }
    }).collect()
}
