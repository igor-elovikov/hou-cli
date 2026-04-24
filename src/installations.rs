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
    pub user_prefs_dir: PathBuf,
    ready: bool,
}

#[derive(Debug)]
pub enum InstalledProduct {
    Houdini(HoudiniInstallation),
    HServer(Installation),
    LicenseServer(Installation),
    HQueueServer(Installation),
    HQueueClient(Installation),
}

pub fn discover_installations() -> Result<Vec<InstalledProduct>> {
    todo!("products discovering")
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
        let user_prefs_dir = Self::user_prefs_dir(&version)?;

        Ok(HoudiniInstallation {
            hfs,
            version,
            user_prefs_dir,
            ready,
        })
    }

    #[cfg(target_os = "linux")]
    fn user_prefs_dir(version: &Version) -> Result<PathBuf> {
        let dirs =
            directories::BaseDirs::new().context("Failed to get user preference directory")?;
        let home = dirs.home_dir();

        let houdini_prefs = home.join(format!("houdini{}.{}", version.major, version.minor));

        Ok(houdini_prefs)
    }

    #[cfg(target_os = "macos")]
    fn user_prefs_dir(version: &Version) -> Result<PathBuf> {
        let dirs =
            directories::BaseDirs::new().context("Failed to get user preference directory")?;
        let pref = dirs.preference_dir();

        let houdini_prefs = pref
            .join("houdini")
            .join(format!("{}.{}", version.major, version.minor));

        Ok(houdini_prefs)
    }

    fn env(&self) -> Result<Vec<(OsString, OsString)>> {
        let bin_path = self.hfs.join("bin");
        let sbin_path = self.hfs.join("sbin");
        let hb = self.hfs.join("bin");
        let hdso = self.hfs.join("..").join("Libraries");
        let hh = self.hfs.join("houdini");
        let hhc = hh.join("config");
        let ht = hh.join("toolkit");
        let hsb = hb.join("sbin");

        let path_env = env_paths_added("PATH", &[bin_path, sbin_path])?;

        Ok(vec![
            ("PATH".into(), path_env),
            ("HFS".into(), self.hfs.clone().into()),
            ("H".into(), self.hfs.clone().into()),
            ("HB".into(), hb.into()),
            ("HDSOP".into(), hdso.into()),
            ("HH".into(), hh.into()),
            ("HHC".into(), hhc.into()),
            ("HT".into(), ht.into()),
            ("HSB".into(), hsb.into()),
        ])
    }

    pub fn launch_houdini(&self) -> Result<ExitStatus> {
        let hou_executable = self.hfs.join("bin").join("houdini");
        self.run(Command::new(hou_executable))
    }

    pub fn run(&self, mut cmd: Command) -> Result<ExitStatus> {
        cmd.envs(self.env()?)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context(format!("Failed to run {:?}", cmd))
    }
}
