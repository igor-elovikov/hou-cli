use crate::hou::Context;
use crate::project::{HouProjectOptions, PROJECT_MARKER, PROJECT_PKGS_DIR};
use anyhow::{Context as _, Result, bail};
use clap::Args;
use serde::Serialize;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct InitCmd {
    /// Optional project directory. If omitted, initializes the current directory.
    pub name: Option<String>,
    /// Houdini version to pin in the project options.
    #[arg(short, long, conflicts_with = "package")]
    pub version: Option<String>,
    /// Initialize a Houdini package instead of a project. Package file name defaults to the directory name.
    #[arg(long, value_name = "NAME", num_args = 0..=1, require_equals = true, default_missing_value = "")]
    pub package: Option<String>,
    /// Point `hpath` straight at `$HOUDINI_PACKAGE_PATH` without a package env var.
    #[arg(long, requires = "package")]
    pub no_package_env: bool,
    /// Skip creating the standard Houdini subdirectories.
    #[arg(long)]
    pub no_layout: bool,
}

const PACKAGE_LAYOUT: &[&str] = &[
    "otls",
    "scripts/python",
    "config/Icons",
    "toolbar",
    "vex/include",
    "ocl/include",
    "viewer_states",
    "viewer_handles",
    "desktop",
    "python_panels",
];

impl InitCmd {
    pub fn run(self, ctx: &Context, version_filter: Option<&str>) -> Result<()> {
        let root = resolve_root(self.name.as_deref())?;
        fs::create_dir_all(&root)
            .with_context(|| format!("Failed to create {}", root.display()))?;

        if let Some(name) = self.package.as_deref() {
            init_package(&root, name, self.no_package_env)?;
            if !self.no_layout {
                create_layout(&root)?;
            }
            println!("Initialized package at {}", root.display());
            return Ok(());
        }

        let marker = root.join(PROJECT_MARKER);
        if marker.exists() {
            bail!("{} already exists", marker.display());
        }

        let houdini_version = match ctx.resolve_houdini(version_filter) {
            Ok(h) => Some(format!("~{}.{}", h.version.major, h.version.minor)),
            Err(e) => {
                if version_filter.is_some() {
                    return Err(e);
                }
                log::warn!(
                    "No Houdini installed; leaving houdini_version empty in project options"
                );
                None
            }
        };

        let options = HouProjectOptions {
            isolated: false,
            houdini_version,
        };

        let hproject = json!({
            "hpath": "$HPROJECT",
            "env": [],
            "hou_project_options": options,
        });
        let body = serde_json::to_string_pretty(&hproject)?;
        fs::write(&marker, format!("{body}\n"))
            .with_context(|| format!("Failed to write {}", marker.display()))?;

        if !self.no_layout {
            create_layout(&root)?;
        }

        let cache = root.join(PROJECT_PKGS_DIR).join("cache");
        fs::create_dir_all(&cache)
            .with_context(|| format!("Failed to create {}", cache.display()))?;

        println!("Initialized project at {}", root.display());
        Ok(())
    }
}

#[derive(Serialize)]
struct PackageFile {
    hpath: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Value>,
}

fn init_package(root: &Path, name: &str, no_env: bool) -> Result<()> {
    let name = match name {
        "" => root
            .file_name()
            .and_then(|n| n.to_str())
            .context("Cannot derive package name from directory; pass --package=<NAME>")?,
        n => n,
    };
    let file = root.join(format!("{name}.json"));
    if file.exists() {
        bail!("{} already exists", file.display());
    }

    let package = if no_env {
        PackageFile {
            hpath: "$HOUDINI_PACKAGE_PATH".into(),
            env: None,
        }
    } else {
        let var = package_env_var(name);
        PackageFile {
            hpath: format!("${var}"),
            env: Some(json!([{ var: "$HOUDINI_PACKAGE_PATH" }])),
        }
    };
    let body = serde_json::to_string_pretty(&package)?;
    fs::write(&file, format!("{body}\n"))
        .with_context(|| format!("Failed to write {}", file.display()))?;
    Ok(())
}

/// Env var name for a package, e.g. `heditor` -> `HEDITOR_PATH`.
fn package_env_var(name: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    format!("{base}_PATH")
}

fn create_layout(root: &Path) -> Result<()> {
    for sub in PACKAGE_LAYOUT {
        let p = root.join(sub);
        fs::create_dir_all(&p).with_context(|| format!("Failed to create {}", p.display()))?;
    }
    Ok(())
}

fn resolve_root(name: Option<&str>) -> Result<PathBuf> {
    let cwd = env::current_dir().context("Failed to read current directory")?;
    Ok(match name {
        Some(n) => cwd.join(n),
        None => cwd,
    })
}
