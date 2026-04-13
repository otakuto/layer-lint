use std::collections::HashMap;

use crate::LayerName;
use crate::feature::config::YamlConfig;
use super::{CrateSetExpr, RuleEntryExpr};

pub struct ConfigExpr {
    pub internal_layers: HashMap<LayerName, Vec<CrateSetExpr>>,
    pub external_layers: HashMap<LayerName, Vec<CrateSetExpr>>,
    pub rules: Vec<RuleEntryExpr>,
}

fn convert_layers(
    raw: std::collections::HashMap<LayerName, Vec<crate::feature::config::YamlCrateSet>>,
) -> anyhow::Result<HashMap<LayerName, Vec<CrateSetExpr>>> {
    raw.into_iter()
        .map(|(k, v)| {
            let members = v
                .into_iter()
                .map(CrateSetExpr::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((k, members))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()
}

impl TryFrom<YamlConfig> for ConfigExpr {
    type Error = anyhow::Error;

    fn try_from(yaml: YamlConfig) -> anyhow::Result<Self> {
        if yaml.version != 0 {
            anyhow::bail!("unsupported config version: {} (expected 0)", yaml.version);
        }
        let internal_layers = convert_layers(yaml.layers.internal)?;
        let external_layers = convert_layers(yaml.layers.external)?;

        let rules = yaml
            .rules
            .into_iter()
            .map(RuleEntryExpr::try_from)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ConfigExpr { internal_layers, external_layers, rules })
    }
}
