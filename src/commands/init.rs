use crate::hou::Context;
use crate::project::{HouProjectOptions, PROJECT_MARKER, PROJECT_PKGS_DIR};
use anyhow::{Context as _, Result, bail};
use clap::Args;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitCmd {
    /// Optional project directory. If omitted, initializes the current directory.
    pub name: Option<String>,
    /// Houdini version to pin in the project options.
    #[arg(short, long)]
    pub version: Option<String>,
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

        for sub in PACKAGE_LAYOUT {
            let p = root.join(sub);
            fs::create_dir_all(&p).with_context(|| format!("Failed to create {}", p.display()))?;
        }

        let cache = root.join(PROJECT_PKGS_DIR).join("cache");
        fs::create_dir_all(&cache)
            .with_context(|| format!("Failed to create {}", cache.display()))?;

        println!("Initialized project at {}", root.display());
        Ok(())
    }
}

fn resolve_root(name: Option<&str>) -> Result<PathBuf> {
    let cwd = env::current_dir().context("Failed to read current directory")?;
    Ok(match name {
        Some(n) => cwd.join(n),
        None => cwd,
    })
}
