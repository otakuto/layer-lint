use crate::LintError;
use crate::feature::expr::ConfigExpr;
use super::internal::{check_undefined_layers, check_layer_cycles};

pub fn check_config_expr(config: &ConfigExpr) -> Vec<LintError> {
    let mut errors = Vec::new();
    errors.extend(check_undefined_layers(config));
    errors.extend(check_layer_cycles(config));
    errors
}
