use anyhow::{Context, Result, bail};
use ini::Ini;
use std::path::{Path, PathBuf};

const SIDEFX_INI: &str = "sidefx.ini";

pub struct Settings {
    path: PathBuf,
    ini: Ini,
}

impl Settings {
    pub fn load(config_dir: &Path) -> Result<Self> {
        let path = config_dir.join(SIDEFX_INI);
        let ini = if path.exists() {
            Ini::load_from_file(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
        } else {
            Ini::new()
        };
        Ok(Self { path, ini })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        self.ini
            .write_to_file(&self.path)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    fn get(&self, key: &str, env_key: &str) -> Option<String> {
        self.ini
            .section(None::<String>)
            .and_then(|s| s.get(key))
            .map(String::from)
            .or_else(|| std::env::var(env_key).ok())
    }

    pub fn username(&self) -> Option<String> {
        self.get("username", "HOU_USERNAME")
    }

    pub fn password(&self) -> Option<String> {
        self.get("password", "HOU_PASSWORD")
    }

    pub fn client_id(&self) -> Option<String> {
        self.get("client_id", "HOU_CLIENT_ID")
    }

    pub fn client_secret(&self) -> Option<String> {
        self.get("client_secret", "HOU_CLIENT_SECRET")
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
        let path = config_dir.join(SIDEFX_INI);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn eulas(&self) -> Vec<String> {
        self.ini
            .section(None::<String>)
            .and_then(|s| s.get("accept_eula"))
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default()
    }

    pub fn set_user_login(&mut self, username: &str, password: &str) {
        self.ini
            .with_section(None::<String>)
            .set("username", username)
            .set("password", password);
    }

    pub fn set_oauth(&mut self, client_id: &str, client_secret: &str) {
        self.ini
            .with_section(None::<String>)
            .set("client_id", client_id)
            .set("client_secret", client_secret);
    }

    /// Adds an EULA date if not already present. Returns true if added.
    pub fn add_eula(&mut self, date: &str) -> bool {
        let mut current = self.eulas();
        if current.iter().any(|d| d == date) {
            return false;
        }
        current.push(date.to_string());
        let joined = current.join(" ");
        self.ini
            .with_section(None::<String>)
            .set("accept_eula", joined);
        true
    }

    pub fn clear_eulas(&mut self) {
        if let Some(s) = self.ini.section_mut(None::<String>) {
            s.remove("accept_eula");
        }
    }
}
