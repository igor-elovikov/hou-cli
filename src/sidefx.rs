mod build;
mod download;
mod products;

use anyhow::{Context, Result, anyhow, bail};
pub use build::BuildsQuery;
pub use download::{BuildDownload, BuildDownloadQuery, BuildSpec};
use indicatif::{ProgressBar, ProgressStyle};
pub use products::{HoudiniLauncher, Platform, Product, Release, Status};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
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

    pub fn install_launcher(
        &self,
        launcher: HoudiniLauncher,
        version: impl Into<String>,
        dest: &Path,
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

        let build = self.download_build(&info, dest)?;
        install_launcher(&build, dest)
    }
}

#[cfg(target_os = "linux")]
fn install_launcher(installer: &Path, dest: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(installer)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(installer, perms)?;

    let status = std::process::Command::new(installer)
        .arg("houdini_launcher")
        .current_dir(dest)
        .status()
        .context("failed to run install_houdini_launcher.sh")?;
    if !status.success() {
        bail!("launcher install script failed with status {status}");
    }

    Ok(dest.join("houdini_launcher"))
}

#[cfg(target_os = "macos")]
fn install_launcher(dmg: &Path, dest: &Path) -> Result<PathBuf> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    let install_dir = dest.join("launcher");
    std::fs::create_dir_all(&install_dir)
        .with_context(|| format!("failed to create {}", install_dir.display()))?;

    pb.set_message("Mounting DMG...");
    let mount_point = dest.join(".launcher_dmg_mount");
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
    if app_dst.exists() {
        std::fs::remove_dir_all(&app_dst)
            .with_context(|| format!("failed to remove {}", app_dst.display()))?;
    }

    pb.set_message("Copying Houdini Launcher (this may take a moment)...");
    let copy_result = std::process::Command::new("cp")
        .arg("-R")
        .arg(&app_src)
        .arg(&install_dir)
        .status()
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
