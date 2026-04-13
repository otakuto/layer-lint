use std::process::Command;

use anyhow::Context;

pub fn load_cargo_metadata_json() -> anyhow::Result<Vec<u8>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("Failed to run 'cargo metadata'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("'cargo metadata' failed:\n{}", stderr);
    }

    Ok(output.stdout)
}
