use crate::installations::{HoudiniInstallation, Installation, InstalledProduct};
use anyhow::{Context, Result, bail};
use is_executable::IsExecutable;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct Installer {
    installer_exe: PathBuf,
    installer_path: PathBuf,
}

pub enum InstallerCommand {
    Install,
    Uninstall,
}

impl InstallerCommand {
    pub fn args(&self) -> Vec<String> {
        match self {
            InstallerCommand::Install => vec!["install".to_owned()],
            InstallerCommand::Uninstall => vec!["uninstall".to_owned()],
        }
    }
}

#[derive(Debug, Deserialize)]
struct Overview {
    installations: BTreeMap<String, OverviewEntry>,
}

#[derive(Debug, Deserialize)]
struct OverviewEntry {
    product: String,
    version: String,
    ready: bool,
}

impl Installer {
    pub fn discover(data_path: &Path) -> Result<Self> {
        let candidates = Self::candidate_paths(data_path);

        for path in &candidates {
            if path.is_executable() {
                return Ok(Self {
                    installer_exe: path.clone(),
                    installer_path: path
                        .parent()
                        .context("No parent path for installer")?
                        .to_path_buf(),
                });
            }
        }

        bail!(
            "No houdini_installer found. Searched:\n{}",
            candidates
                .iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    pub fn run(&self, command: InstallerCommand) -> Result<String> {
        let mut cmd = Command::new(self.installer_exe.clone().into_os_string());

        let stdout = cmd.args(&command.args()).output()?.stdout;

        String::from_utf8(stdout).context("Failed to parse stdout")
    }

    pub fn products(&self) -> Result<Vec<InstalledProduct>> {
        let overview_path = Self::overview_path();

        let overview_data = fs::read_to_string(&overview_path).with_context(|| {
            format!(
                "Failed to read overview file at {}",
                overview_path.display()
            )
        })?;

        let overview: Overview = serde_json::from_str(&overview_data).with_context(|| {
            format!(
                "Failed to parse overview file at {}",
                overview_path.display()
            )
        })?;

        let mut products = Vec::new();

        for (path, entry) in overview.installations {
            let path = path.as_str();
            let version = entry.version.as_str();
            let ready = entry.ready;

            let product = match entry.product.as_str() {
                "Houdini" => {
                    InstalledProduct::Houdini(HoudiniInstallation::new(path, version, ready)?)
                }
                "hserver" => InstalledProduct::HServer(Installation::new(path, version, ready)?),
                "License Server" => {
                    InstalledProduct::LicenseServer(Installation::new(path, version, ready)?)
                }
                "HQueue Server" => {
                    InstalledProduct::HQueueServer(Installation::new(path, version, ready)?)
                }
                "HQueue Client" => {
                    InstalledProduct::HQueueClient(Installation::new(path, version, ready)?)
                }
                other => bail!("Unknown product: {other}"),
            };

            products.push(product);
        }

        Ok(products)
    }

    #[cfg(target_os = "macos")]
    fn overview_path() -> PathBuf {
        PathBuf::from("/Library/Application Support/com.sidefx.launcher/overview.json")
    }

    #[cfg(target_os = "linux")]
    fn overview_path() -> PathBuf {
        unimplemented!("Overview path not implemented for linux")
    }

    #[cfg(target_os = "windows")]
    fn overview_path() -> PathBuf {
        unimplemented!("Overview path not implemented for windows")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    fn overview_path() -> PathBuf {
        unimplemented!("Overview path not implemented for this platform")
    }

    #[cfg(target_os = "macos")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            data_path.join("launcher/Houdini Launcher.app/Contents/MacOS/houdini_installer"),
            PathBuf::from("/Applications/Houdini Launcher.app/Contents/MacOS/houdini_installer"),
        ]
    }

    #[cfg(target_os = "linux")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            data_path.join("installer/houdini_installer"),
            PathBuf::from("/opt/sidefx/launcher/bin/houdini_installer"),
        ]
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn candidate_paths(_data_path: &Path) -> Vec<PathBuf> {
        vec![]
    }
}
