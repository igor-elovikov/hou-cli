use crate::installations::HoudiniInstallation;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const MANIFEST_FILE: &str = "hou-packages-manifest.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub package_path: Vec<PathBuf>,
    #[serde(default)]
    pub hou_package_manifest: BTreeMap<PathBuf, SourceMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceMetadata {
    #[serde(rename = "git")]
    Git(GitMeta),
    #[serde(rename = "folder")]
    Folder(FolderMeta),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitMeta {
    pub url: String,
    pub commit: String,
    pub checksum: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMeta {
    pub path: PathBuf,
}

impl Manifest {
    pub fn path_for(houdini: &HoudiniInstallation) -> PathBuf {
        houdini.user_prefs_dir.join("packages").join(MANIFEST_FILE)
    }

    pub fn load(houdini: &HoudiniInstallation) -> Result<Self> {
        let path = Self::path_for(houdini);
        if !path.exists() {
            log::debug!("Manifest missing at {}, using empty default", path.display());
            return Ok(Self::default());
        }
        log::debug!("Loading manifest from {}", path.display());
        let text = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest at {}", path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse manifest at {}", path.display()))
    }

    pub fn save(&self, houdini: &HoudiniInstallation) -> Result<()> {
        let path = Self::path_for(houdini);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        write_atomic(&path, &serde_json::to_vec_pretty(self)?)?;
        log::debug!("Saved manifest to {}", path.display());
        Ok(())
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("Manifest path has no parent directory")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
    tmp.write_all(bytes)
        .with_context(|| format!("Failed to write manifest to {}", tmp.path().display()))?;
    tmp.as_file_mut()
        .sync_all()
        .context("Failed to fsync manifest")?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist manifest at {}", path.display()))?;
    Ok(())
}
