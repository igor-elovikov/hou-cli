use crate::hou::Context;
use crate::package::checksum::short_url_hash;
use semver::Version;
use std::path::PathBuf;

pub fn cache_root(ctx: &Context, version: &Version) -> PathBuf {
    ctx.data_dir
        .join("packages_cache")
        .join(format!("{}.{}", version.major, version.minor))
}

pub fn install_dir_for(
    ctx: &Context,
    version: &Version,
    url: &str,
    name_hint: Option<&str>,
) -> PathBuf {
    let name = name_hint
        .map(|s| s.to_string())
        .unwrap_or_else(|| derive_repo_name(url));
    let suffix = short_url_hash(url);
    cache_root(ctx, version).join(format!("{name}-{suffix}"))
}

pub fn derive_repo_name(url: &str) -> String {
    let trimmed = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("package");
    trimmed.trim_end_matches(".git").to_string()
}
