use crate::installations::{HoudiniInstallation, InstalledProduct};
use crate::installer::Installer;
use anyhow::Context as _;
use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Context {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub installer: Installer,
    pub products: Vec<InstalledProduct>,
}

impl Context {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", "hou")
            .context("Failed to determine system directory paths")?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        let data_dir = proj_dirs.data_dir().to_path_buf();

        log::info!("Config directory: {}", config_dir.display());
        log::info!("Data directory: {}", data_dir.display());

        // Create folders immediately so they are ready for use
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory at {:?}", config_dir))?;

        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory at {:?}", data_dir))?;

        let installer = Installer::discover(&data_dir)?;
        log::info!("Installer discovered: {:?}", installer);

        let products = installer.products()?;

        log::info!("Products installed: {:#?}", products);

        Ok(Self {
            config_dir,
            data_dir,
            installer,
            products,
        })
    }

    pub fn resolve_houdini(&self, filter: Option<&str>) -> Result<&HoudiniInstallation> {
        let houdinis = self.products.iter().filter_map(|p| match p {
            InstalledProduct::Houdini(h) => Some(h),
            _ => None,
        });

        let selected = match filter {
            None => houdinis.max_by_key(|h| &h.version),
            Some(f) => {
                let normalized = normalize_filter(f);
                let req = semver::VersionReq::parse(&normalized)
                    .with_context(|| format!("Invalid version requirement '{f}'"))?;
                houdinis
                    .filter(|h| req.matches(&h.version))
                    .max_by_key(|h| &h.version)
            }
        };

        selected.ok_or_else(|| match filter {
            None => anyhow!("No Houdini installations found"),
            Some(f) => anyhow!("No Houdini matching '{f}' installation found"),
        })
    }
}

fn normalize_filter(s: &str) -> String {
    if s.chars().next().map_or(false, |c| c.is_ascii_digit()) && !s.contains('*') {
        format!("~{s}")
    } else {
        s.to_string()
    }
}
