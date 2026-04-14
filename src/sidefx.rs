mod build;
mod download;
mod products;

pub use build::{Build, BuildsQuery};
pub use download::{BuildDownload, BuildDownloadQuery, BuildSpec};
pub use products::{Houdini, HoudiniLauncher, LauncherIso, Platform, Product, Release, Status};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::Value;
use ureq::Agent;

const CLIENT_ID: &str = "j6VpXfB18GrkBsvO1SPrr5Z2wxwjjbmS9QiuVGFN";
const CLIENT_SECRET: &str = "ymW6Zeenh5j2xCPtxB4RcDpMXkEDqAF9d0rEJETExiCyx1AqrKAaLFoZqUrXQGETibHtGzQMtGJ0CXKKocTd8bt43C7McEGQpKoJmdD62xg494Reo1HkjiV1btPg7C8S";

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
    pub fn new() -> Result<Self> {
        let agent = Agent::new_with_defaults();
        let token = fetch_token(&agent)?;
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

        let bytes: Vec<u8> = self
            .agent
            .get(&info.download_url)
            .call()
            .context("download request failed")?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_vec()
            .context("failed to read download body")?;

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

        let info = self
            .build_download(
                Product::HoudiniLauncher(launcher),
                version,
                BuildSpec::Production,
            )
            .platform(platform)
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
    let install_dir = dest.join("launcher");
    std::fs::create_dir_all(&install_dir)
        .with_context(|| format!("failed to create {}", install_dir.display()))?;

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
    let copy_result = std::process::Command::new("cp")
        .arg("-R")
        .arg(&app_src)
        .arg(&install_dir)
        .status()
        .context("failed to run cp for Houdini Launcher.app");

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

    Ok(app_dst)
}

fn fetch_token(agent: &Agent) -> Result<String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let creds = STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"));

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
