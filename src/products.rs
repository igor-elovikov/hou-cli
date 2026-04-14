use anyhow::{Context, Result};
use itertools::Itertools;
use semver::Version;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub struct Installation {
    pub path: PathBuf,
    pub version: Version,
    pub ready: bool,
}

#[derive(Debug)]
pub struct HoudiniInstallation {
    hfs: PathBuf,
    pub version: Version,
    ready: bool,
}

#[derive(Debug)]
pub enum Product {
    Houdini(HoudiniInstallation),
    HServer(Installation),
    LicenseServer(Installation),
    HQueueServer(Installation),
    HQueueClient(Installation),
}

fn env_paths_added<S: AsRef<OsStr>>(env_name: S, paths: &[PathBuf]) -> Result<OsString> {
    let path_env = env::var_os(env_name).unwrap_or(OsString::new());

    let env_paths = env::split_paths(&path_env)
        .chain(paths.iter().cloned())
        .unique()
        .collect::<Vec<_>>();

    env::join_paths(env_paths).context("Failed to join PATH environment variable")
}

impl Installation {
    pub fn new(path: &str, version_str: &str, ready: bool) -> Result<Self> {
        Ok(Self {
            path: PathBuf::from(path),
            version: Version::parse(version_str)?,
            ready,
        })
    }
}

impl HoudiniInstallation {
    pub fn new(install_path: &str, version_str: &str, ready: bool) -> Result<HoudiniInstallation> {
        let path = PathBuf::from(install_path);

        let hfs = if cfg!(target_os = "macos") {
            path.join("Frameworks")
                .join("Houdini.framework")
                .join("Resources")
        } else {
            path.clone()
        };

        let version = Version::parse(version_str)?;

        Ok(HoudiniInstallation {
            hfs,
            version,
            ready,
        })
    }

    fn env(&self) -> Result<Vec<(OsString, OsString)>> {
        let bin_path = self.hfs.join("bin");
        let sbin_path = self.hfs.join("sbin");

        let path_env = env_paths_added("PATH", &[bin_path, sbin_path])?;

        Ok(vec![
            ("PATH".into(), path_env),
            ("HFS".into(), self.hfs.clone().into()),
        ])
    }

    pub fn run(&self, mut cmd: Command) -> Result<ExitStatus> {
        cmd.envs(self.env()?)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to start command")
    }
}
