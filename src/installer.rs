use crate::installations::{HoudiniInstallation, Installation, InstalledProduct};
use anyhow::{Context, Result, bail};
use is_executable::IsExecutable;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Subdirectory of the data dir where `hou` installs and discovers the
/// SideFX launcher (both `houdini_installer` and `houdini_launcher` live here).
pub const INSTALLER_DIR: &str = "installer";

/// The `hou`-managed launcher install directory under `data_dir`, used by
/// `setup` and as the fallback for `update`. Matches the local layout that
/// [`Installer::candidate_paths`] discovers and that [`Installer::launcher_dir`]
/// returns for a local install. The shape differs per platform (a bare dir on
/// macOS that holds the `.app`, the launcher dir itself on Linux).
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
    /// Elevates with sudo on unix; the installer writes to system locations.
    /// EULA dates go on the command line; the ini accept_eula key is ignored
    /// by current installers despite being documented.
    /// On Apple Silicon the M1 build option is mandatory: the x86_64-only
    /// installer infers Intel under Rosetta (the GUI checkbox does the same).
    pub fn install_houdini(
        &self,
        version: &str,
        settings_file: &Path,
        eulas: &[String],
    ) -> Result<()> {
        let mut cmd = self.elevated_command("sudo needed to install Houdini to system locations");
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
        let status = cmd.status().context("failed to run houdini_installer")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    /// Path to the discovered houdini_installer executable.
    pub fn path(&self) -> &Path {
        &self.installer_exe
    }

    /// Directory the discovered launcher is installed in, suitable to pass back
    /// to [`crate::sidefx::Client::install_launcher`] as the install target.
    /// `update` reinstalls here so a system launcher (e.g. `/opt/sidefx/launcher`)
    /// is refreshed in place rather than shadowed by a local copy.
    ///
    /// On Linux this is the dir holding `bin/` (the discovered exe is at
    /// `<dir>/bin/houdini_installer`). On macOS it is the dir holding
    /// `Houdini Launcher.app` (exe at `<dir>/Houdini Launcher.app/Contents/MacOS/houdini_installer`).
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
        let mut cmd = self.elevated_command("sudo needed to remove a system Houdini install");
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

    fn elevated_command(&self, reason: &str) -> Command {
        elevated_command(&self.installer_exe, reason)
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

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn candidate_paths(_data_path: &Path) -> Vec<PathBuf> {
        vec![]
    }
}

/// Builds a command that runs `exe` with elevated privileges where needed.
/// Prefers sudo when not root and sudo can actually work: interactively when a
/// tty allows a password prompt, headless only if passwordless sudo is
/// available. Falls back to a direct invocation. A non-empty `reason` is shown
/// when sudo is used to explain the password prompt.
#[cfg(unix)]
pub fn elevated_command(exe: &Path, reason: &str) -> Command {
    use std::io::IsTerminal;

    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false);
    if is_root {
        return Command::new(exe);
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
        if !reason.is_empty() {
            eprintln!("{reason}");
        }
        log::info!("running {} with sudo", exe.display());
        let mut cmd = Command::new("sudo");
        cmd.arg(exe);
        cmd
    } else {
        log::warn!(
            "not root and sudo unavailable; running {} unprivileged",
            exe.display()
        );
        Command::new(exe)
    }
}

#[cfg(not(unix))]
pub fn elevated_command(exe: &Path, _reason: &str) -> Command {
    Command::new(exe)
}
