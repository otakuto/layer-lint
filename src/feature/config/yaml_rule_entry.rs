use serde::Deserialize;

use super::YamlCrateSet;

#[derive(Deserialize)]
pub struct YamlPolicyEntry {
    pub allow: Option<Vec<YamlCrateSet>>,
    pub deny: Option<Vec<YamlCrateSet>>,
    pub ignore: Option<Vec<YamlCrateSet>>,
}

#[derive(Deserialize)]
pub struct YamlRuleEntry {
    #[serde(flatten)]
    pub from: YamlCrateSet,
    #[serde(default)]
    pub internal: Vec<YamlPolicyEntry>,
    #[serde(default)]
    pub external: Vec<YamlPolicyEntry>,
}
