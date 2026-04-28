use crate::installations::HoudiniInstallation;
use crate::package::cache::install_dir_for;
use crate::package::checksum::dir_digest;
use crate::package::git;
use crate::package::manifest::{FolderMeta, GitMeta, Manifest, SourceMetadata};
use crate::package::source::{InstallSource, InstallSpec};
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

pub fn install(
    houdini: &HoudiniInstallation,
    manifest: &mut Manifest,
    cache_root: &Path,
    spec: InstallSpec,
) -> Result<()> {
    match spec.source {
        InstallSource::Git { url, version } => install_git(
            houdini,
            manifest,
            cache_root,
            &url,
            &version,
            spec.name.as_deref(),
        ),
        InstallSource::Folder { path } => install_folder(manifest, path),
    }
}

fn install_git(
    houdini: &HoudiniInstallation,
    manifest: &mut Manifest,
    cache_root: &Path,
    url: &str,
    version: &str,
    name_hint: Option<&str>,
) -> Result<()> {
    let install_dir = install_dir_for(cache_root, &houdini.version, url, name_hint);

    if manifest.hou_package_manifest.contains_key(&install_dir) {
        bail!(
            "Package already installed at {}. Use `package update` or `package uninstall` first.",
            install_dir.display()
        );
    }

    log::info!(
        "Installing git package {url} @ {version} into {}",
        install_dir.display()
    );

    let ref_name = git::ref_kind_from_version(version).as_ref_name();
    let commit = git::clone_at(url, &install_dir, ref_name)?;
    let checksum = dir_digest(&install_dir)?;
    log::debug!("Computed checksum {checksum}");

    let entry = SourceMetadata::Git(GitMeta {
        url: url.to_string(),
        commit,
        checksum,
        version: version.to_string(),
    });

    add_entry(manifest, install_dir.clone(), entry);
    println!("Installed {} at {}", url, install_dir.display());
    Ok(())
}

fn install_folder(manifest: &mut Manifest, path: PathBuf) -> Result<()> {
    if manifest.hou_package_manifest.contains_key(&path) {
        bail!("Package already installed at {}", path.display());
    }
    log::info!("Linking folder {}", path.display());
    let entry = SourceMetadata::Folder(FolderMeta { path: path.clone() });
    add_entry(manifest, path.clone(), entry);
    println!("Linked folder at {}", path.display());
    Ok(())
}

fn add_entry(manifest: &mut Manifest, key: PathBuf, entry: SourceMetadata) {
    if !manifest.package_path.iter().any(|p| p == &key) {
        manifest.package_path.push(key.clone());
    }
    manifest.hou_package_manifest.insert(key, entry);
}
