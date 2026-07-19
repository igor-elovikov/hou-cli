use crate::installations::{HoudiniInstallation, InstalledProduct};
use crate::launcher::Launcher;
use anyhow::{Context as _, bail};
use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use semver::VersionReq;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Context {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub installer: Option<Launcher>,
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

        let mut installer: Option<Launcher> = None;
        let mut products = Vec::new();

        if let Ok(discovered_installer) = Launcher::discover() {
            log::info!("Installer discovered: {:?}", discovered_installer);
            let installed_products = discovered_installer.products()?;
            log::info!("Products installed: {:#?}", installed_products);

            products = installed_products;
            installer = Some(discovered_installer);
        }

        Ok(Self {
            config_dir,
            data_dir,
            installer,
            products,
        })
    }

    pub fn installer(&self) -> Result<&Launcher> {
        self.installer.as_ref().ok_or_else(|| anyhow!("No installer found. Install from sidefx.com or run `hou setup` to install the launcher."))
    }

    /// Installed Houdini builds.
    pub fn houdinis(&self) -> impl Iterator<Item = &HoudiniInstallation> {
        self.products.iter().filter_map(|p| match p {
            InstalledProduct::Houdini(h) => Some(h),
            _ => None,
        })
    }

    pub fn resolve_product(&self, kind: &str, filter: Option<&str>) -> Result<&InstalledProduct> {
        let normalized = normalize_filter(filter);

        let req = VersionReq::parse(&normalized).with_context(|| "Invalid version requirement")?;

        let selected = self
            .products
            .iter()
            .filter(|p| p.kind() == kind)
            .filter(|h| req.matches(h.version()))
            .max_by_key(|h| h.version());

        selected.ok_or_else(|| match filter {
            None => anyhow!("No Houdini installations found"),
            Some(f) => anyhow!("No Houdini matching '{f}' installation found"),
        })
    }
    pub fn resolve_houdini(&self, filter: Option<&str>) -> Result<&HoudiniInstallation> {
        let product = self.resolve_product("houdini", filter)?;

        match product {
            InstalledProduct::Houdini(houdini) => Ok(houdini),
            _ => bail!("No Houdini installations found"),
        }
    }
}

fn normalize_filter(s: Option<&str>) -> String {
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
