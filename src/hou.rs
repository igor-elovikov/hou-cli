use crate::installations::{HoudiniInstallation, InstalledProduct};
use crate::installer::Installer;
use anyhow::Context as _;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Context {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub installer: Option<Installer>,
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

        let mut installer: Option<Installer> = None;
        let mut products = Vec::new();

        if let Ok(discovered_installer) = Installer::discover(&data_dir) {
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

    pub fn installer(&self) -> Result<&Installer> {
        self.installer.as_ref().ok_or_else(|| anyhow!("No installer found. Install from sidefx.com or run `hou setup` to install the launcher."))
    }

    /// Installed Houdini builds.
    pub fn houdinis(&self) -> impl Iterator<Item = &HoudiniInstallation> {
        self.products.iter().filter_map(|p| match p {
            InstalledProduct::Houdini(h) => Some(h),
            _ => None,
        })
    }

    pub fn resolve_houdini(&self, filter: Option<&str>) -> Result<&HoudiniInstallation> {
        let selected = match filter {
            None => self.houdinis().max_by_key(|h| &h.version),
            Some(f) => {
                let normalized = normalize_filter(f);
                let req = semver::VersionReq::parse(&normalized)
                    .with_context(|| format!("Invalid version requirement '{f}'"))?;
                self.houdinis()
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
    // Leave explicit operators (^, >=, etc.) and wildcards untouched.
    if !s.chars().next().map_or(false, |c| c.is_ascii_digit()) || s.contains('*') {
        return s.to_string();
    }
    // A fully specified version (major.minor.patch) must match exactly.
    // Using `~` here would treat e.g. `21.0.559` as `>=21.0.559, <21.1.0`,
    // so `resolve_houdini`'s `max_by_key` would pick a higher patch such as
    // `21.0.729`. Partial versions keep `~` for prefix matching, e.g. `21.0`
    // matches all `21.0.x`.
    if s.split('.').count() >= 3 {
        format!("={s}")
    } else {
        format!("~{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::{Version, VersionReq};

    fn req(filter: &str) -> VersionReq {
        VersionReq::parse(&normalize_filter(filter)).unwrap()
    }

    fn matches(filter: &str, version: &str) -> bool {
        req(filter).matches(&Version::parse(version).unwrap())
    }

    #[test]
    fn full_version_matches_exactly() {
        // Regression: `21.0.559` must not match a higher patch like `21.0.729`.
        assert!(matches("21.0.559", "21.0.559"));
        assert!(!matches("21.0.559", "21.0.729"));
        assert!(!matches("21.0.559", "21.0.558"));
    }

    #[test]
    fn partial_version_matches_prefix() {
        assert!(matches("21.0", "21.0.559"));
        assert!(matches("21.0", "21.0.729"));
        assert!(!matches("21.0", "21.1.0"));

        assert!(matches("21", "21.0.559"));
        assert!(matches("21", "21.9.999"));
        assert!(!matches("21", "22.0.0"));
    }

    #[test]
    fn explicit_operators_and_wildcards_pass_through() {
        assert_eq!(normalize_filter("^21.0"), "^21.0");
        assert_eq!(normalize_filter(">=21.0.0"), ">=21.0.0");
        assert_eq!(normalize_filter("21.0.*"), "21.0.*");
    }
}
