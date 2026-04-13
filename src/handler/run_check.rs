use std::path::Path;

use anyhow::{Context, bail};

use crate::infra::cargo_metadata::{load_cargo_metadata_json, CargoMetadata, metadata_to_dependencies};
use crate::feature::config::load_yaml_config;
use crate::feature::expr::{ConfigExpr, check_config_expr};
use crate::feature::eval::{RuleEvaluator, check_evaluator};
use crate::feature::report::print_errors;
use crate::feature::crate_dependency::check_workspace_dependency;

pub fn run_check(config_path: &Path) -> anyhow::Result<()> {
    // config: file → YamlConfig
    let yaml_config = load_yaml_config(config_path)
        .with_context(|| format!("Failed to load config from '{}'", config_path.display()))?;

    // expr: YamlConfig → ConfigExpr
    let config_expr = ConfigExpr::try_from(yaml_config)?;

    // expr_check: validate
    let errors = check_config_expr(&config_expr);
    if !errors.is_empty() {
        print_errors(&errors)?;
        bail!("{} config errors found", errors.len());
    }

    // load workspace dependencies
    let json = load_cargo_metadata_json()?;
    let metadata: CargoMetadata =
        serde_json::from_slice(&json).context("Failed to parse cargo metadata JSON")?;
    let workspace = metadata_to_dependencies(metadata);

    // crate_dependency_check: validate workspace
    let errors = check_workspace_dependency(&workspace);
    if !errors.is_empty() {
        print_errors(&errors)?;
        bail!("{} workspace errors found", errors.len());
    }

    // eval: ConfigExpr + workspace → RuleEvaluator
    let evaluator = RuleEvaluator::new(config_expr, &workspace);

    // eval_check: RuleEvaluator + workspace → errors
    let errors = check_evaluator(&evaluator, &workspace);

    // report: errors → output
    if !errors.is_empty() {
        print_errors(&errors)?;
        bail!("{} architecture errors found", errors.len());
    }

    Ok(())
}
