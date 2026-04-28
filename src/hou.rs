use crate::installations::{HoudiniInstallation, InstalledProduct};
use crate::installer::Installer;
use anyhow::Context as _;
use anyhow::{Result, anyhow, bail};
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
                let parts: Vec<&str> = f.split('.').collect();
                let (major, minor, patch) = match parts.as_slice() {
                    [maj, min] => (parse_part(maj, f)?, parse_part(min, f)?, None),
                    [maj, min, "*"] => (parse_part(maj, f)?, parse_part(min, f)?, None),
                    [maj, min, pat] => (
                        parse_part(maj, f)?,
                        parse_part(min, f)?,
                        Some(parse_part(pat, f)?),
                    ),
                    _ => bail!(
                        "Invalid version filter '{f}': expected major.minor or major.minor.patch"
                    ),
                };

                houdinis
                    .filter(|h| {
                        h.version.major == major
                            && h.version.minor == minor
                            && patch.map_or(true, |p| h.version.patch == p)
                    })
                    .max_by_key(|h| &h.version)
            }
        };

        selected.ok_or_else(|| match filter {
            None => anyhow!("No Houdini installations found"),
            Some(f) => anyhow!("No Houdini {f} installation found"),
        })
    }
}

fn parse_part(s: &str, full: &str) -> Result<u64> {
    s.parse::<u64>()
        .with_context(|| format!("Invalid version filter '{full}'"))
}
