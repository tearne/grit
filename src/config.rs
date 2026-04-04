use color_eyre::eyre::{bail, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    #[serde(default = "default_diff_tool")]
    pub(crate) diff_tool: String,
    #[serde(default = "default_theme")]
    pub(crate) theme: String,
    #[serde(default = "default_auto_refresh")]
    pub(crate) auto_refresh: u64,
    #[serde(default = "default_preview_split")]
    pub(crate) preview_split: u8,
}

fn default_diff_tool() -> String {
    "difft --color always".to_string()
}

fn default_theme() -> String {
    "autumn".to_string()
}

fn default_auto_refresh() -> u64 {
    10
}

fn default_preview_split() -> u8 {
    50
}

const SCAFFOLD: &str = "\
# Diff tool to invoke for generating diffs. Must be on PATH.
# The tool is called with two file paths; ANSI colour output is expected.
diff_tool = \"difft --color always\"

# Colour theme for the TUI. Available values: default, autumn.
theme = \"autumn\"

# Seconds between automatic refreshes of the file list. Minimum: 1.
auto_refresh = 10

# Percentage of terminal height given to the preview pane. Range: 1–99.
preview_split = 50
";

impl Default for Config {
    fn default() -> Self {
        Config {
            diff_tool: default_diff_tool(),
            theme: default_theme(),
            auto_refresh: default_auto_refresh(),
            preview_split: default_preview_split(),
        }
    }
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("could not determine platform config directory"))?;
        let path = config_dir.join("grit").join("config.toml");
        let config = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            toml::from_str(&raw)?
        } else {
            std::fs::create_dir_all(&config_dir.join("grit"))?;
            std::fs::write(&path, SCAFFOLD)?;
            Config::default()
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        let diff_bin = self.diff_tool.split_whitespace().next().unwrap_or("");
        if diff_bin.is_empty() || which::which(diff_bin).is_err() {
            bail!(
                "diff tool '{}' not found on PATH — install it or set a different tool in ~/.config/grit/config.toml",
                self.diff_tool
            );
        }
        if self.auto_refresh < 1 {
            bail!("auto_refresh must be at least 1 second");
        }
        if self.preview_split < 1 || self.preview_split > 99 {
            bail!("preview_split must be between 1 and 99");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> Config {
        toml::from_str(toml).expect("valid toml")
    }

    #[test]
    fn scaffold_matches_defaults() {
        let config: Config = toml::from_str(SCAFFOLD).expect("scaffold is valid toml");
        let defaults = Config::default();
        assert_eq!(config.diff_tool, defaults.diff_tool);
        assert_eq!(config.theme, defaults.theme);
        assert_eq!(config.auto_refresh, defaults.auto_refresh);
        assert_eq!(config.preview_split, defaults.preview_split);
    }

    #[test]
    fn loads_auto_refresh_and_preview_split() {
        let config = parse("auto_refresh = 30\npreview_split = 40\n");
        assert_eq!(config.auto_refresh, 30);
        assert_eq!(config.preview_split, 40);
    }

    #[test]
    fn rejects_auto_refresh_zero() {
        let config = parse("diff_tool = \"diff\"\nauto_refresh = 0\n");
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_preview_split_zero() {
        let config = parse("diff_tool = \"diff\"\npreview_split = 0\n");
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_preview_split_one_hundred() {
        let config = parse("diff_tool = \"diff\"\npreview_split = 100\n");
        assert!(config.validate().is_err());
    }
}
