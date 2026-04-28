use crate::package::checksum::short_url_hash;
use semver::Version;
use std::path::{Path, PathBuf};

pub fn version_cache_dir(cache_root: &Path, version: &Version) -> PathBuf {
    cache_root.join(format!("{}.{}", version.major, version.minor))
}

pub fn install_dir_for(
    cache_root: &Path,
    version: &Version,
    url: &str,
    name_hint: Option<&str>,
) -> PathBuf {
    let name = name_hint
        .map(|s| s.to_string())
        .unwrap_or_else(|| derive_repo_name(url));
    let suffix = short_url_hash(url);
    version_cache_dir(cache_root, version).join(format!("{name}-{suffix}"))
}

pub fn derive_repo_name(url: &str) -> String {
    let trimmed = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("package");
    trimmed.trim_end_matches(".git").to_string()
}
