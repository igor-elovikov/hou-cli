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
use crate::package::manifest::{Manifest, SourceMetadata};
use crate::package::source::InstallSpec;
use crate::package::uninstall::uninstall;
use crate::package::update::{UpdateTarget, update};
use anyhow::{Context as _, Result};

pub struct Packages<'a> {
    ctx: &'a Context,
    houdini: &'a HoudiniInstallation,
    manifest: Manifest,
}

impl<'a> Packages<'a> {
    pub fn open(ctx: &'a Context, houdini: &'a HoudiniInstallation) -> Result<Self> {
        let manifest = Manifest::load(houdini)?;
        Ok(Self {
            ctx,
            houdini,
            manifest,
        })
    }

    pub fn install(&mut self, spec: InstallSpec) -> Result<()> {
        install(self.ctx, self.houdini, &mut self.manifest, spec)?;
        self.manifest.save(self.houdini)
    }

    pub fn uninstall(&mut self, key_or_name: &str) -> Result<()> {
        uninstall(&mut self.manifest, key_or_name)?;
        self.manifest.save(self.houdini)
    }

    pub fn update(&mut self, key_or_name: &str, target: UpdateTarget) -> Result<()> {
        update(
            self.ctx,
            self.houdini,
            &mut self.manifest,
            key_or_name,
            target,
        )?;
        self.manifest.save(self.houdini)
    }

    pub fn list(&self) -> &Manifest {
        &self.manifest
    }

    pub fn check(&mut self, repair: bool) -> Result<CheckReport> {
        log::info!(
            "Checking {} package(s) (repair={repair})",
            self.manifest.hou_package_manifest.len()
        );
        let mut report = CheckReport::default();
        let keys: Vec<_> = self.manifest.hou_package_manifest.keys().cloned().collect();
        for key in keys {
            let entry = self.manifest.hou_package_manifest.get(&key).cloned();
            let Some(SourceMetadata::Git(git)) = entry else {
                log::debug!("Skipping non-git entry {}", key.display());
                report.skipped.push(key);
                continue;
            };
            if !key.exists() {
                log::warn!("Missing install dir {}", key.display());
                report.missing.push(key.clone());
                if repair {
                    repair_git(self, &key, &git)?;
                    report.repaired.push(key);
                }
                continue;
            }
            let digest = dir_digest(&key)?;
            if digest == git.checksum {
                log::debug!("OK {}", key.display());
                report.ok.push(key);
            } else {
                log::warn!("Checksum mismatch at {}", key.display());
                report.mismatched.push(key.clone());
                if repair {
                    repair_git(self, &key, &git)?;
                    report.repaired.push(key);
                }
            }
        }
        if repair {
            self.manifest.save(self.houdini)?;
        }
        Ok(report)
    }
}

fn repair_git(
    pkgs: &mut Packages<'_>,
    key: &std::path::Path,
    git: &manifest::GitMeta,
) -> Result<()> {
    use crate::package::manifest::{GitMeta, SourceMetadata};
    log::info!("Repairing {} (version {})", key.display(), git.version);
    if key.exists() {
        std::fs::remove_dir_all(key)
            .with_context(|| format!("Failed to clear {}", key.display()))?;
    }
    let ref_name = git::ref_kind_from_version(&git.version).as_ref_name();
    let commit = git::clone_at(&git.url, key, ref_name)?;
    let checksum = dir_digest(key)?;
    pkgs.manifest.hou_package_manifest.insert(
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
pub struct CheckReport {
    pub ok: Vec<std::path::PathBuf>,
    pub mismatched: Vec<std::path::PathBuf>,
    pub missing: Vec<std::path::PathBuf>,
    pub repaired: Vec<std::path::PathBuf>,
    pub skipped: Vec<std::path::PathBuf>,
}

pub use uninstall::resolve_key as resolve_package_key;
