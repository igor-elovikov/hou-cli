use crate::package::checksum::dir_digest;
use crate::package::git;
use crate::package::manifest::{GitMeta, Manifest, SourceMetadata};
use crate::package::patch;
use crate::package::uninstall::resolve_key;
use anyhow::{Result, anyhow, bail};
use semver::Version;

pub enum UpdateTarget {
    Auto,
    Latest,
    Tag(String),
}

pub fn update(
    manifest: &mut Manifest,
    key_or_name: &str,
    target: UpdateTarget,
    no_patch: bool,
) -> Result<()> {
    let key = resolve_key(manifest, key_or_name)?;
    let entry = manifest
        .hou_package_manifest
        .get(&key)
        .cloned()
        .ok_or_else(|| anyhow!("No package at {}", key.display()))?;

    let git_meta = match entry {
        SourceMetadata::Git(g) => g,
        SourceMetadata::Folder(_) => {
            bail!("Update only applies to git packages");
        }
    };

    log::info!(
        "Updating {} (current: {} @ {})",
        key.display(),
        git_meta.version,
        git_meta.commit
    );

    let new_version = match target {
        UpdateTarget::Latest => "latest".to_string(),
        UpdateTarget::Tag(t) => t,
        UpdateTarget::Auto => resolve_auto_semver(&git_meta)?,
    };
    log::info!("Resolved update target: {}", new_version);

    let ref_name = git::ref_kind_from_version(&new_version).as_ref_name();

    let commit = if key.join(".git").is_dir() {
        git::fetch_update(&key, ref_name)?
    } else {
        git::clone_at(&git_meta.url, &key, ref_name)?
    };
    if !no_patch {
        patch::patch_dir(&key)?;
    }
    let checksum = dir_digest(&key)?;

    manifest.hou_package_manifest.insert(
        key.clone(),
        SourceMetadata::Git(GitMeta {
            url: git_meta.url,
            commit,
            checksum,
            version: new_version.clone(),
        }),
    );
    println!("Updated {} to {}", key.display(), new_version);
    Ok(())
}

fn resolve_auto_semver(current: &GitMeta) -> Result<String> {
    let current_ver = parse_semver(&current.version).ok_or_else(|| {
        anyhow!(
            "Current version '{}' is not semver; re-run with --tag <T> or --latest",
            current.version
        )
    })?;

    let tags = git::list_remote_tags(&current.url)?;
    let best = tags
        .iter()
        .filter_map(|t| parse_semver(t).map(|v| (t.clone(), v)))
        .filter(|(_, v)| v > &current_ver)
        .max_by(|(_, a), (_, b)| a.cmp(b));

    match best {
        Some((tag, _)) => Ok(tag),
        None => Err(anyhow!(
            "No semver tag newer than {} on {}",
            current.version,
            current.url
        )),
    }
}

fn parse_semver(raw: &str) -> Option<Version> {
    Version::parse(raw.trim_start_matches('v')).ok()
}
