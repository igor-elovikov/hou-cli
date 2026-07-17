use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CONFIG_TOML: &str = "config.toml";

/// Config keys accepted by `hou config`, in display order.
pub const KEYS: [&str; 2] = ["launcher_path", "use_api_keys"];

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_api_keys: Option<bool>,
}

impl Config {
    pub fn load(config_dir: &Path) -> Result<Self> {
        let path = Self::path(config_dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self, config_dir: &Path) -> Result<()> {
        let path = Self::path(config_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("failed to serialize config")?;
        std::fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join(CONFIG_TOML)
    }

    /// Value of `key` as a display string, or None if unset.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(match key {
            "launcher_path" => self
                .launcher_path
                .as_ref()
                .map(|p| p.display().to_string()),
            "use_api_keys" => self.use_api_keys.map(|v| v.to_string()),
            _ => bail!(unknown_key(key)),
        })
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "launcher_path" => self.launcher_path = Some(PathBuf::from(value)),
            "use_api_keys" => self.use_api_keys = Some(parse_bool(value)?),
            _ => bail!(unknown_key(key)),
        }
        Ok(())
    }

    pub fn unset(&mut self, key: &str) -> Result<()> {
        match key {
            "launcher_path" => self.launcher_path = None,
            "use_api_keys" => self.use_api_keys = None,
            _ => bail!(unknown_key(key)),
        }
        Ok(())
    }
}

fn unknown_key(key: &str) -> String {
    format!("unknown config key '{key}'; valid keys: {}", KEYS.join(", "))
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => bail!("expected a boolean (true/false), got '{value}'"),
    }
}
