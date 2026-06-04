use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CREDENTIALS_TOML: &str = "credentials.toml";

#[derive(Default, Serialize, Deserialize)]
struct Credentials {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    accept_eula: Vec<String>,
}

pub struct Settings {
    path: PathBuf,
    credentials: Credentials,
}

impl Settings {
    pub fn load(config_dir: &Path) -> Result<Self> {
        let path = config_dir.join(CREDENTIALS_TOML);
        let credentials = if path.exists() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?
        } else {
            Credentials::default()
        };
        Ok(Self { path, credentials })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let text =
            toml::to_string_pretty(&self.credentials).context("failed to serialize credentials")?;
        std::fs::write(&self.path, text)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    fn get(&self, value: &Option<String>, env_key: &str) -> Option<String> {
        value.clone().or_else(|| std::env::var(env_key).ok())
    }

    pub fn username(&self) -> Option<String> {
        self.get(&self.credentials.username, "HOU_USERNAME")
    }

    pub fn password(&self) -> Option<String> {
        self.get(&self.credentials.password, "HOU_PASSWORD")
    }

    pub fn client_id(&self) -> Option<String> {
        self.get(&self.credentials.client_id, "HOU_CLIENT_ID")
    }

    pub fn client_secret(&self) -> Option<String> {
        self.get(&self.credentials.client_secret, "HOU_CLIENT_SECRET")
    }

    /// Returns the OAuth credentials, or an error if they aren't set.
    pub fn require_oauth(&self) -> Result<(String, String)> {
        match (self.client_id(), self.client_secret()) {
            (Some(id), Some(secret)) => Ok((id, secret)),
            _ => bail!(
                "no SideFX OAuth credentials; run `hou login oauth <client_id> <client_secret>` or set HOU_CLIENT_ID/HOU_CLIENT_SECRET"
            ),
        }
    }

    /// Removes the settings file if it exists.
    pub fn delete(config_dir: &Path) -> Result<bool> {
        let path = config_dir.join(CREDENTIALS_TOML);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn eulas(&self) -> Vec<String> {
        self.credentials.accept_eula.clone()
    }

    pub fn set_user_login(&mut self, username: &str, password: &str) {
        self.credentials.username = Some(username.to_string());
        self.credentials.password = Some(password.to_string());
    }

    pub fn set_oauth(&mut self, client_id: &str, client_secret: &str) {
        self.credentials.client_id = Some(client_id.to_string());
        self.credentials.client_secret = Some(client_secret.to_string());
    }

    /// Adds an EULA date if not already present. Returns true if added.
    pub fn add_eula(&mut self, date: &str) -> bool {
        if self.credentials.accept_eula.iter().any(|d| d == date) {
            return false;
        }
        self.credentials.accept_eula.push(date.to_string());
        true
    }

    pub fn clear_eulas(&mut self) {
        self.credentials.accept_eula.clear();
    }
}
