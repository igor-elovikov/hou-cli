mod build;
mod download;
mod products;

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::elevated_command::try_elevated_command;
#[cfg(target_os = "linux")]
use crate::elevated_command::try_elevated_command_with_path;
use anyhow::{Context, Result, anyhow, bail};
pub use build::BuildsQuery;
pub use download::{BuildDownload, BuildDownloadQuery, BuildSpec};
use indicatif::{ProgressBar, ProgressStyle};
pub use products::{Houdini, HoudiniLauncher, Platform, Product, Release, Status};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
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
    pub fn install_launcher(
        &self,
        launcher: HoudiniLauncher,
        version: impl Into<String>,
        staging_dir: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        let platform = Platform::host()?;

        let info = self
            .build_download(
                Product::HoudiniLauncher(launcher),
                version,
                BuildSpec::Production,
            )
            .platform(platform)
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

    let reason = format!(
        "sudo needed to install the launcher to {}",
        target_dir.display()
    );

    try_elevated_command_with_path(installer, &["-q".into(), name.into()], &reason, &parent)?;

    Ok(target_dir.to_path_buf())
}

#[cfg(target_os = "macos")]
fn install_launcher(dmg: &Path, target_dir: &Path) -> Result<PathBuf> {
    // `target_dir` is the directory that holds `Houdini Launcher.app`.
    let install_dir = target_dir;
    std::fs::create_dir_all(install_dir)
        .with_context(|| format!("failed to create {}", install_dir.display()))?;

    // Mount the DMG next to the downloaded file, not under the install target.
    let staging_dir = dmg.parent().unwrap_or(install_dir);

    println!("Mounting DMG...");
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
    if app_dst.exists() {
        match std::fs::remove_dir_all(&app_dst) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                let reason = format!(
                    "sudo needed to remove the old launcher and copy the new one to {}",
                    install_dir.display()
                );
                let app_dst_arg = app_dst.as_os_str().to_os_string();
                let status =
                    try_elevated_command(Path::new("rm"), &["-rf".into(), app_dst_arg], &reason)
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

    println!("Copying Houdini Launcher (this may take a moment)...");
    let copy_result = try_elevated_command(
        Path::new("cp"),
        &[
            "-R".into(),
            app_src.as_os_str().to_os_string(),
            install_dir.as_os_str().to_os_string(),
        ],
        "Copying Houdini Launcher requires admin privileges",
    )
    .context("failed to run cp for Houdini Launcher.app");

    println!("Unmounting and cleaning up...");
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

    println!("Successfully installed Houdini Launcher!");

    Ok(app_dst)
}

#[cfg(target_os = "windows")]
fn install_launcher(installer: &Path, target_dir: &Path) -> Result<PathBuf> {
    if !target_dir.is_absolute() {
        bail!(
            "launcher install path must be absolute on Windows, got {}",
            target_dir.display()
        );
    }

    let needs_elevation = location_needs_admin(target_dir);
    if !needs_elevation {
        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("failed to create {}", target_dir.display()))?;
    }

    try_elevated_command(
        installer,
        &[
            OsString::from("/S"),
            OsString::from(format!("/D={}", target_dir.display())),
        ],
        "Launcher installation requires administration privileges",
    )?;

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
