use std::path::Path;

use color_eyre::eyre::{bail, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    #[serde(default = "default_diff_tool")]
    pub(crate) diff_tool: String,
}

fn default_diff_tool() -> String {
    "difft --color always".to_string()
}

impl Config {
    pub(crate) fn load(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join(".grit.toml");
        let config = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            toml::from_str(&raw)?
        } else {
            Config { diff_tool: default_diff_tool() }
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        let diff_bin = self.diff_tool.split_whitespace().next().unwrap_or("");
        if diff_bin.is_empty() || which::which(diff_bin).is_err() {
            bail!(
                "diff tool '{}' not found on PATH — install it or set a different tool in .grit.toml",
                self.diff_tool
            );
        }
        Ok(())
    }
}
