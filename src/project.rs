use crate::package::manifest::{HouProjectOptions, Manifest};
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

pub const PROJECT_MARKER: &str = "hproject.json";
pub const PROJECT_PKGS_DIR: &str = "hou-packages";
pub const PROJECT_MANIFEST: &str = "hproject-manifest.json";

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub manifest_path: PathBuf,
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
        let manifest_path = root.join(PROJECT_PKGS_DIR).join(PROJECT_MANIFEST);
        let manifest = Manifest::load_from(&manifest_path)?;
        log::debug!("Found project at {}", root.display());
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            manifest_path,
        })
    }

    pub fn options(&self) -> Option<&HouProjectOptions> {
        self.manifest.hou_project_options.as_ref()
    }

    pub fn houdini_version(&self) -> Option<&str> {
        self.options()
            .and_then(|o| o.houdini_version.as_deref())
    }

    pub fn isolated(&self) -> bool {
        self.options().map_or(false, |o| o.isolated)
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
