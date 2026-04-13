use serde::Deserialize;

use crate::{CrateName, LayerName, RegexPattern};

#[derive(Deserialize)]
pub struct YamlCrateSet {
    #[serde(rename = "crate")]
    pub crate_name: Option<CrateName>,
    pub regex: Option<RegexPattern>,
    pub layer: Option<LayerName>,
    pub exclude: Option<Vec<YamlCrateSet>>,
}
