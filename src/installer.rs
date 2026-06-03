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

    /// Installs a Houdini build with stdio inherited from the terminal.
    /// Elevates with sudo on unix; the installer writes to system locations.
    /// EULA dates go on the command line; the ini accept_eula key is ignored
    /// by current installers despite being documented.
    /// On Apple Silicon the M1 build option is mandatory: the x86_64-only
    /// installer infers Intel under Rosetta (the GUI checkbox does the same).
    pub fn install_houdini(&self, version: &str, settings_file: &Path, eulas: &[String]) -> Result<()> {
        let mut cmd = self.elevated_command();
        cmd.arg("install")
            .args(["--product", "Houdini", "--version", version])
            .arg("--settings-file")
            .arg(settings_file);
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            cmd.args(["--build-option", "M1"]);
        }
        for date in eulas {
            cmd.arg("--accept-EULA").arg(date);
        }
        let status = cmd
            .status()
            .context("failed to run houdini_installer")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    /// Returns the launcher version reported by houdini_installer.
    pub fn version(&self) -> Result<semver::Version> {
        let output = Command::new(&self.installer_exe)
            .arg("--version")
            .output()
            .context("failed to run houdini_installer")?;
        let text = String::from_utf8_lossy(&output.stdout);
        let version_str = text
            .split_whitespace()
            .last()
            .context("unexpected houdini_installer version output")?;
        semver::Version::parse(version_str)
            .with_context(|| format!("failed to parse launcher version from '{}'", text.trim()))
    }

    /// Uninstalls the product at the given install directory.
    pub fn uninstall(&self, installdir: &Path) -> Result<()> {
        let mut cmd = self.elevated_command();
        let status = cmd
            .arg("uninstall")
            .arg(installdir)
            .status()
            .context("failed to run houdini_installer")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    /// Prefers sudo when not root and sudo can actually work:
    /// interactively when a tty allows a password prompt, headless only if
    /// passwordless sudo is available. Falls back to a direct invocation.
    #[cfg(unix)]
    fn elevated_command(&self) -> Command {
        use std::io::IsTerminal;

        let is_root = Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false);
        if is_root {
            return Command::new(&self.installer_exe);
        }

        let sudo_usable = if std::io::stdin().is_terminal() {
            Command::new("sudo")
                .arg("-V")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            Command::new("sudo")
                .args(["-n", "true"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        if sudo_usable {
            eprintln!("Running houdini_installer with sudo");
            let mut cmd = Command::new("sudo");
            cmd.arg(&self.installer_exe);
            cmd
        } else {
            log::warn!("not root and sudo unavailable; running houdini_installer unprivileged");
            Command::new(&self.installer_exe)
        }
    }

    #[cfg(not(unix))]
    fn elevated_command(&self) -> Command {
        Command::new(&self.installer_exe)
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
