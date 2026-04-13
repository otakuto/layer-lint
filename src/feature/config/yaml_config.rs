use std::collections::HashMap;

use serde::Deserialize;

use crate::LayerName;
use super::{YamlCrateSet, YamlRuleEntry};

#[derive(Deserialize)]
pub struct YamlLayers {
    #[serde(default)]
    pub internal: HashMap<LayerName, Vec<YamlCrateSet>>,
    #[serde(default)]
    pub external: HashMap<LayerName, Vec<YamlCrateSet>>,
}

#[derive(Deserialize)]
pub struct YamlConfig {
    pub version: u32,
    pub layers: YamlLayers,
    pub rules: Vec<YamlRuleEntry>,
}
