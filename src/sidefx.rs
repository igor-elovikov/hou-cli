mod build;
mod download;
mod products;

use std::path::{Path, PathBuf};
pub use build::{Build, BuildsQuery};
pub use download::{BuildDownload, BuildSpec, BuildDownloadQuery};
pub use products::{
    Houdini, HoudiniLauncher, LauncherIso, Platform, Product, Release, Status,
};

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

    pub(super) fn call(&self, method: &str, args: Value, kwargs: Value) -> Result<Value> {
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
        dest: &Path,
    ) -> Result<PathBuf> {
        if cfg!(not(target_os = "linux")) {
            bail!("install_launcher currently supports Linux only");
        }

        let info = self
            .build_download(
                Product::HoudiniLauncher(launcher),
                "21.0",
                BuildSpec::Production,
            )
            .platform(Platform::Linux)
            .send()?;

        let installer = self.download_build(&info, dest)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&installer)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&installer, perms)?;
        }

        let status = std::process::Command::new(&installer)
            .arg("houdini_launcher")
            .current_dir(dest)
            .status()
            .context("failed to run install_houdini_launcher.sh")?;
        if !status.success() {
            bail!("launcher install script failed with status {status}");
        }

        Ok(dest.join("launcher"))
    }
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
