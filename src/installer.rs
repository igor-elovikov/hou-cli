use crate::elevated_command::try_elevated_command;
use crate::installations::{HoudiniInstallation, Installation, InstalledProduct};
use anyhow::{Context, Result, bail};
use is_executable::IsExecutable;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Subdirectory of the data dir where `hou` installs and discovers the
/// SideFX launcher (both `houdini_installer` and `houdini_launcher` live here).
pub const INSTALLER_DIR: &str = "installer";

/// The `hou`-managed launcher install directory under `data_dir`
pub fn default_launcher_dir(data_dir: &Path) -> PathBuf {
    let base = data_dir.join(INSTALLER_DIR);
    if cfg!(target_os = "macos") {
        base
    } else {
        base.join("houdini_launcher")
    }
}

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
    pub fn launcher_dir(&self) -> Option<PathBuf> {
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

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    fn overview_path() -> PathBuf {
        unimplemented!("Overview path not implemented for this platform")
    }

    #[cfg(target_os = "macos")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            data_path
                .join(INSTALLER_DIR)
                .join("Houdini Launcher.app/Contents/MacOS/houdini_installer"),
            PathBuf::from("/Applications/Houdini Launcher.app/Contents/MacOS/houdini_installer"),
        ]
    }

    #[cfg(target_os = "linux")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            data_path
                .join(INSTALLER_DIR)
                .join("houdini_launcher/bin/houdini_installer"),
            PathBuf::from("/opt/sidefx/launcher/bin/houdini_installer"),
        ]
    }

    #[cfg(target_os = "windows")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            data_path
                .join(INSTALLER_DIR)
                .join("houdini_launcher/bin/houdini_installer.exe"),
            PathBuf::from(
                r"C:\Program Files\Side Effects Software\Launcher\bin\houdini_installer.exe",
            ),
        ]
    }
}
