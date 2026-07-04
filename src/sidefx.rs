mod build;
mod download;
mod products;

use anyhow::{Context, Result, anyhow, bail};
pub use build::BuildsQuery;
pub use download::{BuildDownload, BuildDownloadQuery, BuildSpec};
use indicatif::{ProgressBar, ProgressStyle};
pub use products::{Houdini, HoudiniLauncher, Platform, Product, Release, Status};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::time::Duration;
use ureq::Agent;

const TOKEN_URL: &str = "https://www.sidefx.com/oauth2/application_token";
const API_URL: &str = "https://www.sidefx.com/api/";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub struct Client {
    agent: Agent,
    token: String,
}

impl Client {
    pub fn new(client_id: &str, client_secret: &str) -> Result<Self> {
        let agent = Agent::new_with_defaults();
        let token = fetch_token(&agent, client_id, client_secret)?;
        Ok(Self { agent, token })
    }

    pub fn call(&self, method: &str, args: Value, kwargs: Value) -> Result<Value> {
        let payload = serde_json::to_string(&serde_json::json!([method, args, kwargs]))?;

        let response: Value = self
            .agent
            .post(API_URL)
            .header("Authorization", &format!("Bearer {}", self.token))
            .send_form([("json", payload.as_str())])
            .context("API request failed")?
            .body_mut()
            .read_json()
            .context("failed to parse API response")?;

        Ok(response)
    }

    pub fn builds(&self, product: Product) -> BuildsQuery<'_> {
        BuildsQuery::new(self, product)
    }

    pub fn build_download(
        &self,
        product: Product,
        version: impl Into<String>,
        build: BuildSpec,
    ) -> BuildDownloadQuery<'_> {
        BuildDownloadQuery::new(self, product, version.into(), build)
    }

    pub fn download_build(&self, info: &BuildDownload, dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
        let file_path = dir.join(&info.filename);

        // 1. Start the request
        let mut response = self
            .agent
            .get(&info.download_url)
            .call()
            .context("download request failed")?;

        // 2. Setup Progress Bar
        let pb = ProgressBar::new(info.size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")?);
        // .progress_chars("#>-"));

        let mut bytes = Vec::with_capacity(info.size as usize);
        let mut reader = pb.wrap_read(response.body_mut().as_reader());

        reader
            .read_to_end(&mut bytes)
            .context("failed to read download body")?;

        pb.finish_with_message("Download complete");

        if bytes.len() as u64 != info.size {
            bail!("size mismatch: expected {}, got {}", info.size, bytes.len());
        }
        let digest = format!("{:x}", md5::compute(&bytes));
        if digest != info.hash {
            bail!("hash mismatch: expected {}, got {}", info.hash, digest);
        }

        std::fs::write(&file_path, &bytes)
            .with_context(|| format!("failed to write {}", file_path.display()))?;
        Ok(file_path)
    }

    /// Downloads the launcher installer into `staging_dir` (must be writable)
    /// and installs it into `target_dir`. `target_dir` is the launcher directory
    /// itself on Linux (e.g. `data_dir/installer/houdini_launcher` or
    /// `/opt/sidefx/launcher`) and the directory that holds `Houdini Launcher.app`
    /// on macOS. System targets are installed with elevation; see
    /// [`crate::installer::elevated_command`].
    pub fn install_launcher(
        &self,
        launcher: HoudiniLauncher,
        version: impl Into<String>,
        staging_dir: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        let platform = Platform::host()?;

        let launcher_platform = match platform {
            Platform::Macos | Platform::MacosxArm64 => Platform::Macos,
            other => other,
        };

        let info = self
            .build_download(
                Product::HoudiniLauncher(launcher),
                version,
                BuildSpec::Production,
            )
            .platform(launcher_platform)
            .send()?;

        let build = self.download_build(&info, staging_dir)?;
        install_launcher(&build, target_dir)
    }
}

#[cfg(target_os = "linux")]
fn install_launcher(installer: &Path, target_dir: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(installer)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(installer, perms)?;

    // The installer extracts the launcher into the directory named by its
    // argument, resolved against the working directory. Splitting the target
    // into parent + name lets us land it at `data_dir/installer/houdini_launcher`
    // for a local install or `/opt/sidefx/launcher` for a system one.
    let parent = target_dir
        .parent()
        .with_context(|| format!("launcher target {} has no parent", target_dir.display()))?;
    let name = target_dir.file_name().with_context(|| {
        format!(
            "launcher target {} has no final component",
            target_dir.display()
        )
    })?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;

    let status = crate::installer::elevated_command(
        installer,
        &format!(
            "sudo needed to install the launcher to {}",
            target_dir.display()
        ),
    )
    .arg(name)
    .current_dir(parent)
    .status()
    .context("failed to run install_houdini_launcher.sh")?;
    if !status.success() {
        bail!("launcher install script failed with status {status}");
    }

    Ok(target_dir.to_path_buf())
}

#[cfg(target_os = "macos")]
fn install_launcher(dmg: &Path, target_dir: &Path) -> Result<PathBuf> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    // `target_dir` is the directory that holds `Houdini Launcher.app`.
    let install_dir = target_dir;
    std::fs::create_dir_all(install_dir)
        .with_context(|| format!("failed to create {}", install_dir.display()))?;

    // Mount the DMG next to the downloaded file, not under the install target.
    let staging_dir = dmg.parent().unwrap_or(install_dir);

    pb.set_message("Mounting DMG...");
    let mount_point = staging_dir.join(".launcher_dmg_mount");
    if mount_point.exists() {
        std::fs::remove_dir_all(&mount_point).ok();
    }
    std::fs::create_dir_all(&mount_point)
        .with_context(|| format!("failed to create {}", mount_point.display()))?;

    let status = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-quiet", "-mountpoint"])
        .arg(&mount_point)
        .arg(dmg)
        .status()
        .context("failed to run hdiutil attach")?;

    if !status.success() {
        bail!("hdiutil attach failed with status {status}");
    }

    let app_src = mount_point.join("Houdini Launcher.app");
    let app_dst = install_dir.join("Houdini Launcher.app");
    // System installs (e.g. /Applications) are root-owned; escalate when a
    // plain removal is denied and reuse the elevation for the copy.
    let mut elevate = false;
    if app_dst.exists() {
        match std::fs::remove_dir_all(&app_dst) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                elevate = true;
                let reason = format!(
                    "sudo needed to remove the old launcher and copy the new one to {}",
                    install_dir.display()
                );
                let status = pb
                    .suspend(|| {
                        crate::installer::elevated_command(Path::new("rm"), &reason)
                            .arg("-rf")
                            .arg(&app_dst)
                            .status()
                    })
                    .context("failed to run rm for Houdini Launcher.app")?;
                if !status.success() {
                    bail!(
                        "failed to remove {} (rm exited with {status})",
                        app_dst.display()
                    );
                }
            }
            other => other.with_context(|| format!("failed to remove {}", app_dst.display()))?,
        }
    }

    pb.set_message("Copying Houdini Launcher (this may take a moment)...");
    let run_cp = || {
        let mut cmd = if elevate {
            // The rm above already explained the elevation.
            crate::installer::elevated_command(Path::new("cp"), "")
        } else {
            std::process::Command::new("cp")
        };
        cmd.arg("-R").arg(&app_src).arg(install_dir).status()
    };
    let copy_result = if elevate {
        pb.suspend(run_cp)
    } else {
        run_cp()
    }
    .context("failed to run cp for Houdini Launcher.app");

    pb.set_message("Unmounting and cleaning up...");
    let detach_status = std::process::Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(&mount_point)
        .status();
    std::fs::remove_dir_all(&mount_point).ok();

    let copy_status = copy_result?;
    if !copy_status.success() {
        bail!("cp of Houdini Launcher.app failed with status {copy_status}");
    }
    match detach_status {
        Ok(s) if !s.success() => bail!("hdiutil detach failed with status {s}"),
        Err(e) => return Err(e).context("failed to run hdiutil detach"),
        _ => {}
    }

    std::fs::remove_file(dmg).with_context(|| format!("failed to remove {}", dmg.display()))?;

    pb.finish_and_clear();
    println!("Successfully installed Houdini Launcher!");

    Ok(app_dst)
}

#[cfg(target_os = "windows")]
fn install_launcher(installer: &Path, target_dir: &Path) -> Result<PathBuf> {
    use std::os::windows::process::CommandExt;

    // NSIS requires the target to be an absolute path; `/D=` rejects relative
    // ones silently. `default_launcher_dir` already hands us an absolute path,
    // but a caller-supplied one (e.g. a discovered system install) might not be.
    if !target_dir.is_absolute() {
        bail!(
            "launcher install path must be absolute on Windows, got {}",
            target_dir.display()
        );
    }

    // The installer's manifest is `asInvoker`, so it only needs elevation when
    // its target isn't writable by the current user (e.g. under Program Files).
    // Probe actual write access rather than pre-creating the directory: for an
    // update over an existing system launcher the directory already exists, so a
    // `create_dir_all` success would be a false negative and the silent install
    // would then fail without a UAC prompt.
    let needs_elevation = location_needs_admin(target_dir);
    if !needs_elevation {
        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("failed to create {}", target_dir.display()))?;
    }

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(if needs_elevation {
        "Installing Houdini Launcher (approve the administrator prompt)..."
    } else {
        "Installing Houdini Launcher..."
    });

    // The launcher installer is an NSIS package: `/S` runs it silently and `/D=`
    // sets the install directory. `/D=` must be the last argument and must NOT be
    // quoted, even when the path contains spaces (NSIS reads everything after the
    // `=` verbatim to the end of the command line).
    let args = format!("/S /D={}", target_dir.display());
    let status = pb.suspend(|| {
        if needs_elevation {
            run_elevated(installer, &args)
        } else {
            // Rust would otherwise wrap an argument containing spaces in quotes,
            // so pass the whole `/S /D=...` string through `raw_arg` verbatim.
            let mut cmd = std::process::Command::new(installer);
            cmd.raw_arg(&args);
            cmd.status()
                .context("failed to run install-houdini-launcher.exe")
        }
    })?;

    if !status.success() {
        // `run_elevated` maps a declined UAC prompt to ERROR_CANCELLED (1223).
        if needs_elevation && status.code() == Some(1223) {
            pb.finish_and_clear();
            bail!("launcher install was cancelled at the administrator (UAC) prompt");
        }
        bail!("launcher installer failed with status {status}");
    }

    pb.finish_and_clear();
    println!("Successfully installed Houdini Launcher!");

    // Clean up the downloaded installer to match the macOS behaviour. It lives in
    // the (writable) staging dir, so this succeeds even for an elevated install.
    std::fs::remove_file(installer)
        .with_context(|| format!("failed to remove {}", installer.display()))
        .ok();

    Ok(target_dir.to_path_buf())
}

/// Whether installing into `target_dir` requires Administrator rights, decided
/// by probing write access to the deepest existing ancestor (the location the
/// installer will actually create files in). Errors other than a permission
/// denial are treated as "no elevation" so the installer surfaces the real
/// failure with its own diagnostics.
#[cfg(target_os = "windows")]
fn location_needs_admin(target_dir: &Path) -> bool {
    let mut base = target_dir;
    loop {
        if base.exists() {
            break;
        }
        match base.parent() {
            Some(parent) => base = parent,
            None => return false,
        }
    }

    // `NamedTempFile` creates a uniquely named file in `base` and removes it on
    // drop, so this leaves nothing behind whether or not the probe succeeds.
    match tempfile::NamedTempFile::new_in(base) {
        Ok(_) => false,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => true,
        Err(_) => false,
    }
}

/// Runs `exe args` elevated via a UAC prompt and waits for it to finish,
/// returning its exit status. Uses `Start-Process -Verb RunAs`, which is what
/// triggers the consent dialog (a plain `CreateProcess` on an `asInvoker`
/// binary can't self-elevate). The whole argument string is passed as a single
/// `-ArgumentList` value so PowerShell forwards it verbatim, preserving the
/// unquoted `/D=` path NSIS requires. A declined prompt is reported as exit
/// code 1223 (ERROR_CANCELLED).
#[cfg(target_os = "windows")]
fn run_elevated(exe: &Path, args: &str) -> Result<std::process::ExitStatus> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    // Single-quote for PowerShell; a literal quote is escaped by doubling it.
    let quote = |s: &str| s.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         try {{ $p = Start-Process -FilePath '{exe}' -ArgumentList '{args}' -Verb RunAs -Wait -PassThru }} \
         catch {{ exit 1223 }}; \
         exit $p.ExitCode",
        exe = quote(&exe.display().to_string()),
        args = quote(args),
    );

    // `-EncodedCommand` takes base64 of the UTF-16LE script, sidestepping every
    // layer of command-line quoting between here and PowerShell.
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = STANDARD.encode(utf16);

    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
        .arg(encoded)
        .status()
        .context("failed to launch elevated installer via PowerShell")
}

fn fetch_token(agent: &Agent, client_id: &str, client_secret: &str) -> Result<String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let creds = STANDARD.encode(format!("{client_id}:{client_secret}"));

    let resp: TokenResponse = agent
        .post(TOKEN_URL)
        .header("Authorization", &format!("Basic {creds}"))
        .send_form([("grant_type", "client_credentials")])
        .map_err(|e| anyhow!("token request failed: {e}"))?
        .body_mut()
        .read_json()
        .context("failed to parse token response")?;

    Ok(resp.access_token)
}
