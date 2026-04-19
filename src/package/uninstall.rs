use crate::package::manifest::{Manifest, SourceMetadata};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;

pub fn uninstall(manifest: &mut Manifest, key_or_name: &str) -> Result<()> {
    let key = resolve_key(manifest, key_or_name)?;
    let entry = manifest
        .hou_package_manifest
        .remove(&key)
        .ok_or_else(|| anyhow!("No package at {}", key.display()))?;

    manifest.package_path.retain(|p| p != &key);

    if let SourceMetadata::Git(_) = &entry {
        if key.exists() {
            log::info!("Deleting cache dir {}", key.display());
            fs::remove_dir_all(&key)
                .map_err(|e| anyhow!("Failed to remove {}: {}", key.display(), e))?;
        }
    } else {
        log::debug!("Leaving {} in place (local source)", key.display());
    }
    println!("Uninstalled {}", key.display());
    Ok(())
}

pub fn resolve_key(manifest: &Manifest, key_or_name: &str) -> Result<PathBuf> {
    let direct = PathBuf::from(key_or_name);
    if manifest.hou_package_manifest.contains_key(&direct) {
        return Ok(direct);
    }
    let matches: Vec<PathBuf> = manifest
        .hou_package_manifest
        .keys()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == key_or_name || n.starts_with(&format!("{key_or_name}-")))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    match matches.len() {
        0 => Err(anyhow!("No package matching '{key_or_name}'")),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "Ambiguous name '{key_or_name}': matches {} packages",
            matches.len()
        )),
    }
}
