use crate::LintError;
use crate::feature::eval::RuleEvaluator;
use crate::feature::crate_dependency::WorkspaceDependency;
use super::internal::{check_rules, check_unused_allows, check_unused_ignores, check_uncovered_crates};

pub fn check_evaluator(evaluator: &RuleEvaluator, workspace: &WorkspaceDependency) -> Vec<LintError> {
    let dep_map = workspace.as_dep_map();
    let mut errors = Vec::new();
    errors.extend(check_rules(evaluator, &dep_map));
    errors.extend(check_unused_allows(evaluator, &dep_map));
    errors.extend(check_unused_ignores(evaluator, &dep_map));
    errors.extend(check_uncovered_crates(evaluator, &dep_map));
    errors
}
