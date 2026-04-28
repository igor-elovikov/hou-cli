use crate::package::manifest::Manifest;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_MARKER: &str = "hproject.json";
pub const PROJECT_PKGS_DIR: &str = "hou-packages";
pub const PROJECT_MANIFEST: &str = "hproject-manifest.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HouProjectOptions {
    #[serde(default)]
    pub isolated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub houdini_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HProjectFile {
    #[serde(default)]
    hou_project_options: Option<HouProjectOptions>,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub manifest_path: PathBuf,
    pub options: Option<HouProjectOptions>,
}

impl Project {
    pub fn discover(start: &Path) -> Result<Option<Self>> {
        let abs = absolutize(start)?;
        let mut current = abs.as_path();
        loop {
            if current.join(PROJECT_MARKER).is_file() {
                return Ok(Some(Self::load(current)?));
            }
            match current.parent() {
                Some(p) => current = p,
                None => return Ok(None),
            }
        }
    }

    fn load(root: &Path) -> Result<Self> {
        let marker = root.join(PROJECT_MARKER);
        let text = fs::read_to_string(&marker)
            .with_context(|| format!("Failed to read {}", marker.display()))?;
        let parsed: HProjectFile = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse {}", marker.display()))?;

        let manifest_path = root.join(PROJECT_PKGS_DIR).join(PROJECT_MANIFEST);
        let manifest = Manifest::load_from(&manifest_path)?;

        log::debug!("Found project at {}", root.display());
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            manifest_path,
            options: parsed.hou_project_options,
        })
    }

    pub fn houdini_version(&self) -> Option<&str> {
        self.options
            .as_ref()
            .and_then(|o| o.houdini_version.as_deref())
    }

    pub fn isolated(&self) -> bool {
        self.options.as_ref().map_or(false, |o| o.isolated)
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root.join(PROJECT_PKGS_DIR)
    }
}

fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = env::current_dir().context("Failed to read current directory")?;
        Ok(cwd.join(path))
    }
}
