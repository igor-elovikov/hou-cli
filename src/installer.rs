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
        let mut args: Vec<OsString> = vec![
            "install".into(),
            "--product".into(),
            "Houdini".into(),
            "--version".into(),
            version.into(),
            "--settings-file".into(),
            settings_file.as_os_str().to_os_string(),
        ];
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            args.push("--build-option".into());
            args.push("M1".into());
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
        let args: Vec<OsString> = vec!["uninstall".into(), installdir.as_os_str().to_os_string()];
        let status = self.run_installer(&args, "sudo needed to remove a system Houdini install")?;
        if !status.success() {
            bail!("houdini_installer failed with status {status}");
        }
        Ok(())
    }

    /// Runs `houdini_installer` with `args`, elevating as the platform requires:
    /// sudo on unix, and a UAC prompt on Windows when we aren't already elevated
    /// (Houdini installs into `C:\Program Files` and writes logs beside the
    /// launcher, both of which need Administrator rights).
    #[cfg(unix)]
    fn run_installer(&self, args: &[OsString], reason: &str) -> Result<ExitStatus> {
        elevated_command(&self.installer_exe, reason)
            .args(args)
            .status()
            .context("failed to run houdini_installer")
    }

    #[cfg(windows)]
    fn run_installer(&self, args: &[OsString], reason: &str) -> Result<ExitStatus> {
        if is_elevated() {
            // Already Administrator: run directly so output stays in this
            // terminal. `elevated_command` is a plain `Command` on Windows.
            elevated_command(&self.installer_exe, reason)
                .args(args)
                .status()
                .context("failed to run houdini_installer")
        } else {
            eprintln!("Requesting Administrator privileges to run the Houdini installer (approve the prompt)...");
            run_elevated(&self.installer_exe, args)
        }
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

/// Whether the current process is already running with Administrator rights,
/// probed by trying to create (and immediately drop) a file under System32 —
/// a location only administrators can write to. A false negative merely costs
/// an extra UAC prompt, so this errs toward "not elevated".
#[cfg(windows)]
fn is_elevated() -> bool {
    let system32 = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32");
    tempfile::NamedTempFile::new_in(system32).is_ok()
}

/// Runs `exe args` elevated through a UAC prompt and waits for it, returning
/// the child's exit status. A declined prompt surfaces as exit code 1223
/// (ERROR_CANCELLED).
///
/// An elevated process can't share our console, which would normally push its
/// output into a separate window that vanishes on exit (hiding any error). To
/// keep everything in this one terminal we elevate a hidden `cmd.exe` that runs
/// the installer with its output redirected to a temp file, then tail that file
/// live into our stdout until the process exits.
///
/// The command line is built with the standard `CommandLineToArgvW` quoting
/// rules and handed to `Start-Process` as a single string (Windows PowerShell's
/// `Start-Process` does not quote `-ArgumentList` array elements with spaces).
#[cfg(windows)]
fn run_elevated(exe: &Path, args: &[OsString]) -> Result<ExitStatus> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::process::Stdio;

    // A user-owned temp file the elevated process writes and we read back. The
    // handle is closed immediately (only the path is kept) so `cmd`'s `>` redirect
    // doesn't hit a sharing violation; it's deleted when `log_path` drops.
    let log_path = tempfile::Builder::new()
        .prefix("hou-install-")
        .suffix(".log")
        .tempfile()
        .context("failed to create installer log file")?
        .into_temp_path();

    // Inner command for `cmd /c`: "installer" arg1 arg2 ... > "log" 2>&1
    let mut inner = win_quote(&exe.display().to_string());
    for a in args {
        inner.push(' ');
        inner.push_str(&win_quote(&a.to_string_lossy()));
    }
    inner.push_str(" > ");
    inner.push_str(&win_quote(&log_path.display().to_string()));
    inner.push_str(" 2>&1");
    // `cmd /c "..."` strips the outer quote pair and runs the rest verbatim.
    let cmd_line = format!("/c \"{inner}\"");

    // Single-quote for PowerShell; a literal quote doubles. `-WindowStyle Hidden`
    // keeps the elevated cmd window from flashing since output goes to the file.
    let ps_quote = |s: &str| s.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         try {{ $p = Start-Process -FilePath 'cmd.exe' -ArgumentList '{args}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru }} \
         catch {{ exit 1223 }}; \
         exit $p.ExitCode",
        args = ps_quote(&cmd_line),
    );

    // `-EncodedCommand` takes base64 of the UTF-16LE script, sidestepping every
    // layer of command-line quoting between here and PowerShell.
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = STANDARD.encode(utf16);

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
        .arg(encoded)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to launch elevated houdini_installer via PowerShell")?;

    // Tail the log into our stdout until the elevated process exits.
    let drain = |pos: &mut u64| {
        let Ok(mut f) = std::fs::File::open(&log_path) else {
            return;
        };
        if f.seek(SeekFrom::Start(*pos)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if let Ok(n) = f.read_to_end(&mut buf)
            && n > 0
        {
            let out = std::io::stdout();
            let mut lock = out.lock();
            let _ = lock.write_all(&buf);
            let _ = lock.flush();
            *pos += n as u64;
        }
    };

    let mut pos = 0u64;
    let status = loop {
        drain(&mut pos);
        match child.try_wait().context("failed to wait on elevated installer")? {
            Some(status) => {
                drain(&mut pos); // flush anything written just before exit
                break status;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    };

    Ok(status)
}

/// Quotes a single argument per the `CommandLineToArgvW` rules so a spawned
/// process parses it back as one argument (backslashes are only doubled when
/// they precede a quote or the closing quote).
#[cfg(windows)]
fn win_quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::from('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        if c == '\\' {
            // Hold backslashes until we know whether a quote follows them.
            backslashes += 1;
        } else {
            if c == '"' {
                // Double the pending backslashes and escape this quote.
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
            } else {
                // Backslashes not before a quote stay literal.
                out.extend(std::iter::repeat_n('\\', backslashes));
            }
            backslashes = 0;
            out.push(c);
        }
    }
    // Trailing backslashes precede the closing quote, so double them.
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

#[cfg(all(test, windows))]
mod tests {
    use super::win_quote;

    #[test]
    fn plain_args_are_not_quoted() {
        assert_eq!(win_quote("install"), "install");
        assert_eq!(win_quote("--version"), "--version");
        // A path without spaces keeps its backslashes and stays bare.
        assert_eq!(win_quote(r"C:\dir\file"), r"C:\dir\file");
    }

    #[test]
    fn spaces_get_wrapped_in_quotes() {
        assert_eq!(win_quote(""), r#""""#);
        assert_eq!(
            win_quote(r"C:\Users\test\App Data\settings.ini"),
            r#""C:\Users\test\App Data\settings.ini""#
        );
    }

    #[test]
    fn trailing_backslashes_are_doubled_before_closing_quote() {
        // `a b\` -> "a b\\" so the closing quote isn't escaped.
        assert_eq!(win_quote(r"a b\"), r#""a b\\""#);
    }

    #[test]
    fn embedded_quotes_and_preceding_backslashes_are_escaped() {
        assert_eq!(win_quote(r#"a"b"#), r#""a\"b""#);
        // Backslash before a quote is doubled, then the quote is escaped.
        assert_eq!(win_quote(r#"a\"b"#), r#""a\\\"b""#);
    }
}
