use anyhow::anyhow;
use crate::{CrateName, LayerName, RegexPattern};
use crate::feature::config::YamlCrateSet;

#[derive(Debug, Clone)]
pub enum CrateSetExpr {
    Crate(CrateName),
    Regex(RegexPattern),
    Layer(LayerName),
    Exclude(Vec<CrateSetExpr>),
}

impl std::fmt::Display for CrateSetExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrateSetExpr::Crate(name) => write!(f, "{}", name.0),
            CrateSetExpr::Regex(pattern) => write!(f, "{}", pattern.0),
            CrateSetExpr::Layer(name) => write!(f, "layer:{}", name.0),
            CrateSetExpr::Exclude(_) => write!(f, "exclude:..."),
        }
    }
}

impl TryFrom<YamlCrateSet> for CrateSetExpr {
    type Error = anyhow::Error;

    fn try_from(yaml: YamlCrateSet) -> anyhow::Result<Self> {
        match (yaml.crate_name, yaml.regex, yaml.layer, yaml.exclude) {
            (Some(name), None, None, None) => Ok(CrateSetExpr::Crate(name)),
            (None, Some(pattern), None, None) => Ok(CrateSetExpr::Regex(pattern)),
            (None, None, Some(name), None) => Ok(CrateSetExpr::Layer(name)),
            (None, None, None, Some(excludes)) => {
                let excluded = excludes
                    .into_iter()
                    .map(CrateSetExpr::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(CrateSetExpr::Exclude(excluded))
            }
            (None, None, None, None) => {
                Err(anyhow!("must have one of 'crate', 'regex', 'layer', or 'exclude'"))
            }
            _ => Err(anyhow!("must have exactly one of 'crate', 'regex', 'layer', or 'exclude'")),
        }
    }
}
