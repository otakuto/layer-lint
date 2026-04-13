use std::path::Path;

use anyhow::Context;

use super::YamlConfig;

pub fn load_yaml_config(path: &Path) -> anyhow::Result<YamlConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let yaml: YamlConfig =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML config")?;
    Ok(yaml)
}
