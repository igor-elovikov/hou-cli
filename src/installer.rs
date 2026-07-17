use crate::elevated_command::try_elevated_command;
use crate::installations::{HoudiniInstallation, Installation, InstalledProduct};
use anyhow::{bail, Context, Result};
use is_executable::IsExecutable;
#[cfg(target_os = "windows")]
use known_folders::{get_known_folder_path, KnownFolder};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub struct Launcher {
    installer_exe: PathBuf,
    launcher_exe: PathBuf,
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

impl Launcher {
    pub fn discover() -> Result<Self> {
        let installer_exe = Self::default_path();

        if installer_exe.is_executable() {
            let launcher_exe = installer_exe.with_file_name(if cfg!(windows) {
                "houdini_launcher.exe"
            } else {
                "houdini_launcher"
            });

            if launcher_exe.is_executable() {
                return Ok(Self {
                    installer_exe: installer_exe.clone(),
                    launcher_exe: launcher_exe.clone(),
                });
            }
        }

        bail!(
            "No Houdini Launcher found here: {}",
            installer_exe.display()
        );
    }

    /// Installs a Houdini build with stdio inherited from the terminal.
    pub fn install_houdini(
        &self,
        version: &str,
        settings_file: &Path,
        eulas: &[String],
    ) -> Result<()> {
        let mut args: Vec<OsString> = vec![
            "install".into(),
            "--product".into(),
            "Houdini".into(),
            "--version".into(),
            version.into(),
            "--upgrade-hserver-if-needed".into(),
            "--settings-file".into(),
            settings_file.as_os_str().to_os_string(),
        ];

        if cfg!(target_os = "linux") {
            let semver = semver::Version::parse(version)?;
            if semver.major >= 22 {
                args.push("--platform".into());
                if cfg!(target_arch = "aarch64") {
                    args.push("linux_arm64_gcc14.2".into());
                } else {
                    args.push("linux_x86_64_gcc14.2".into());
                }
            }
        }

        for date in eulas {
            args.push("--accept-EULA".into());
            args.push(date.into());
        }
        let status =
            self.run_installer(&args, "sudo needed to install Houdini to system locations")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    /// Path to the discovered houdini_installer executable.
    pub fn path(&self) -> &Path {
        &self.installer_exe
    }

    /// Returns the launcher directory
    pub fn current_install_path(&self) -> Option<PathBuf> {
        let depth = if cfg!(target_os = "macos") { 4 } else { 2 };
        self.installer_exe
            .ancestors()
            .nth(depth)
            .map(Path::to_path_buf)
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
        let args: Vec<OsString> = vec!["uninstall".into(), installdir.as_os_str().to_os_string()];
        let status = self.run_installer(&args, "sudo needed to remove a system Houdini install")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    fn run_installer(&self, args: &[OsString], reason: &str) -> Result<ExitStatus> {
        try_elevated_command(&self.installer_exe, args, reason)
    }

    pub fn run_launcher(&self, args: &[String]) -> Result<ExitStatus> {
        Command::new(&self.launcher_exe)
            .args(args)
            .status()
            .with_context(|| format!("Failed to run launcher at {}", self.launcher_exe.display()))
    }

    pub fn run_installer_bare(&self, args: &[String]) -> Result<ExitStatus> {
        Command::new(&self.installer_exe)
            .args(args)
            .status()
            .with_context(|| {
                format!(
                    "Failed to run installer at {}",
                    self.installer_exe.display()
                )
            })
    }

    pub fn products(&self) -> Result<Vec<InstalledProduct>> {
        let overview_path = self.overview_path()?;

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
    fn overview_path(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(
            "/Library/Application Support/com.sidefx.launcher/overview.json",
        ))
    }

    #[cfg(target_os = "linux")]
    fn overview_path(&self) -> Result<PathBuf> {
        Ok(self
            .installer_exe
            .parent()
            .context("Can't find overview directory in launcher")?
            .parent()
            .context("Can't find overview directory in launcher")?
            .join("data/overview.json"))
    }

    #[cfg(target_os = "windows")]
    fn overview_path(&self) -> Result<PathBuf> {
        Ok(self
            .installer_exe
            .parent()
            .context("Can't find overview directory in launcher")?
            .parent()
            .context("Can't find overview directory in launcher")?
            .join("data/overview.json"))
    }

    #[cfg(target_os = "macos")]
    pub fn install_path() -> PathBuf {
        PathBuf::from("/Applications")
    }
    #[cfg(target_os = "linux")]
    pub fn install_path() -> PathBuf {
        PathBuf::from("/opt/sidefx")
    }

    #[cfg(target_os = "windows")]
    pub fn install_path() -> PathBuf {
        get_known_folder_path(KnownFolder::ProgramFiles)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
    }

    #[cfg(target_os = "macos")]
    pub fn default_path() -> PathBuf {
        PathBuf::from("/Applications/Houdini Launcher.app/Contents/MacOS/houdini_installer")
    }

    #[cfg(target_os = "linux")]
    pub fn default_path() -> PathBuf {
        PathBuf::from("/opt/sidefx/launcher/bin/houdini_installer")
    }

    #[cfg(target_os = "windows")]
    pub fn default_path() -> PathBuf {
        let program_files = get_known_folder_path(KnownFolder::ProgramFiles)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));

        program_files
            .join("Side Effects Software")
            .join("Launcher")
            .join("bin")
            .join("houdini_installer.exe")
    }
}
