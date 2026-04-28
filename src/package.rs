pub mod cache;
pub mod checksum;
pub mod git;
pub mod install;
pub mod manifest;
pub mod source;
pub mod uninstall;
pub mod update;

use crate::hou::Context;
use crate::installations::HoudiniInstallation;
use crate::package::checksum::dir_digest;
use crate::package::install::install;
use crate::package::manifest::{GitMeta, Manifest, SourceMetadata};
use crate::package::source::InstallSpec;
use crate::package::uninstall::uninstall;
use crate::package::update::{UpdateTarget, update};
use crate::project::Project;
use anyhow::{Context as _, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Project,
}

pub struct Packages<'a> {
    pub kind: ScopeKind,
    pub houdini: &'a HoudiniInstallation,
    manifest: Manifest,
    manifest_path: PathBuf,
    cache_root: PathBuf,
}

impl<'a> Packages<'a> {
    pub fn open_global(ctx: &'a Context, houdini: &'a HoudiniInstallation) -> Result<Self> {
        let manifest_path = Manifest::path_for(houdini);
        let cache_root = ctx.data_dir.join("packages_cache");
        let manifest = Manifest::load_from(&manifest_path)?;
        Ok(Self {
            kind: ScopeKind::Global,
            houdini,
            manifest,
            manifest_path,
            cache_root,
        })
    }

    pub fn open_project(houdini: &'a HoudiniInstallation, project: &Project) -> Result<Self> {
        let manifest_path = project.manifest_path.clone();
        let cache_root = project.packages_dir().join("cache");
        let manifest = Manifest::load_from(&manifest_path)?;
        Ok(Self {
            kind: ScopeKind::Project,
            houdini,
            manifest,
            manifest_path,
            cache_root,
        })
    }

    fn save(&self) -> Result<()> {
        self.manifest.save_to(&self.manifest_path)
    }

    pub fn install(&mut self, spec: InstallSpec) -> Result<()> {
        install(self.houdini, &mut self.manifest, &self.cache_root, spec)?;
        self.save()
    }

    pub fn uninstall(&mut self, key_or_name: &str) -> Result<()> {
        uninstall(&mut self.manifest, key_or_name)?;
        self.save()
    }

    pub fn update(&mut self, key_or_name: &str, target: UpdateTarget) -> Result<()> {
        update(&mut self.manifest, key_or_name, target)?;
        self.save()
    }

    pub fn list(&self) -> &Manifest {
        &self.manifest
    }

    pub fn sync(&mut self) -> Result<SyncReport> {
        log::info!(
            "Syncing {} package(s)",
            self.manifest.hou_package_manifest.len()
        );
        let mut report = SyncReport::default();
        let keys: Vec<_> = self.manifest.hou_package_manifest.keys().cloned().collect();
        for key in keys {
            let entry = self.manifest.hou_package_manifest.get(&key).cloned();
            let Some(SourceMetadata::Git(g)) = entry else {
                log::debug!("Skipping non-git entry {}", key.display());
                report.skipped.push(key);
                continue;
            };
            if !key.exists() {
                log::warn!("Missing {}, reinstalling", key.display());
                redownload(&mut self.manifest, &key, &g)?;
                report.repaired.push(key);
                continue;
            }
            let digest = dir_digest(&key)?;
            if digest == g.checksum {
                log::debug!("OK {}", key.display());
                report.ok.push(key);
            } else {
                log::warn!("Checksum mismatch at {}, reinstalling", key.display());
                redownload(&mut self.manifest, &key, &g)?;
                report.repaired.push(key);
            }
        }
        self.save()?;
        Ok(report)
    }
}

fn redownload(manifest: &mut Manifest, key: &Path, git: &GitMeta) -> Result<()> {
    log::info!("Reinstalling {} ({})", key.display(), git.version);
    if key.exists() {
        std::fs::remove_dir_all(key)
            .with_context(|| format!("Failed to clear {}", key.display()))?;
    }
    let ref_name = git::ref_kind_from_version(&git.version).as_ref_name();
    let commit = git::clone_at(&git.url, key, ref_name)?;
    let checksum = dir_digest(key)?;
    manifest.hou_package_manifest.insert(
        key.to_path_buf(),
        SourceMetadata::Git(GitMeta {
            url: git.url.clone(),
            commit,
            checksum,
            version: git.version.clone(),
        }),
    );
    Ok(())
}

#[derive(Default, Debug)]
pub struct SyncReport {
    pub ok: Vec<PathBuf>,
    pub repaired: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}
