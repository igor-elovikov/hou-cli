use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use anyhow::{Context, Result};
use itertools::Itertools;

pub fn env_paths_added<S: AsRef<OsStr>>(env_name: S, paths: &[PathBuf]) -> Result<OsString> {
    let path_env = env::var_os(env_name).unwrap_or(OsString::new());

    let env_paths = env::split_paths(&path_env)
        .chain(paths.iter().cloned())
        .unique()
        .collect::<Vec<_>>();

    env::join_paths(env_paths).context("Failed to join PATH environment variable")
}

pub fn env_paths_prepended<S: AsRef<OsStr>>(env_name: S, paths: &[PathBuf]) -> Result<OsString> {
    let path_env = env::var_os(env_name).unwrap_or(OsString::new());

    let env_paths = paths
        .iter()
        .cloned()
        .chain(env::split_paths(&path_env))
        .unique()
        .collect::<Vec<_>>();

    env::join_paths(env_paths).context("Failed to join env path variable")
}

pub fn normalize_version_filter(s: Option<&str>) -> String {
    if let Some(s) = s {
        // Leave explicit operators (^, >=, etc.) and wildcards untouched.
        if !s.chars().next().map_or(false, |c| c.is_ascii_digit()) || s.contains('*') {
            s.to_string()
        }
        // A fully specified version (major.minor.patch) must match exactly.
        // Partial versions keep `~` for prefix matching, e.g. `21.0 matches all `21.0.x`.
        else if s.split('.').count() >= 3 {
            format!("={s}")
        } else {
            format!("~{s}")
        }
    } else {
        "*".to_owned()
    }
}