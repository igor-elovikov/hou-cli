use crate::installations::{HoudiniInstallation, InstalledProduct};
use crate::installer::Installer;
use anyhow::Context as _;
use anyhow::Result;
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

        Ok(Self {
            config_dir,
            data_dir,
            installer,
            products,
        })
    }

    pub fn latest_houdini(&self) -> Result<&HoudiniInstallation> {
        self.products
            .iter()
            .filter_map(|p| match p {
                InstalledProduct::Houdini(h) => Some(h),
                _ => None,
            })
            .max_by_key(|h| &h.version)
            .context("No Houdini installations found")
    }
}
