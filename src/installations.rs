use crate::project::Project;
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

fn env_paths_prepended<S: AsRef<OsStr>>(env_name: S, paths: &[PathBuf]) -> Result<OsString> {
    let path_env = env::var_os(env_name).unwrap_or(OsString::new());

    let env_paths = paths
        .iter()
        .cloned()
        .chain(env::split_paths(&path_env))
        .unique()
        .collect::<Vec<_>>();

    env::join_paths(env_paths).context("Failed to join env path variable")
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

    fn env(&self, project: Option<&Project>) -> Result<Vec<(OsString, OsString)>> {
        let bin_path = self.hfs.join("bin");
        let sbin_path = self.hfs.join("sbin");
        let hb = self.hfs.join("bin");
        let hdso = self.hfs.join("..").join("Libraries");
        let hh = self.hfs.join("houdini");
        let hhc = hh.join("config");
        let ht = hh.join("toolkit");
        let hsb = hb.join("sbin");

        let path_env = env_paths_added("PATH", &[bin_path, sbin_path])?;

        let mut env: Vec<(OsString, OsString)> = vec![
            ("PATH".into(), path_env),
            ("HFS".into(), self.hfs.clone().into()),
            ("H".into(), self.hfs.clone().into()),
            ("HB".into(), hb.into()),
            ("HDSOP".into(), hdso.into()),
            ("HH".into(), hh.into()),
            ("HHC".into(), hhc.into()),
            ("HT".into(), ht.into()),
            ("HSB".into(), hsb.into()),
        ];

        let global_packages_enabled = match project {
            Some(p) => {
                env.push(("HPROJECT".into(), p.root.clone().into()));
                env.push((
                    "HOUDINI_PACKAGE_DIR".into(),
                    env_paths_prepended(
                        "HOUDINI_PACKAGE_DIR",
                        &[p.root.clone(), p.packages_dir()],
                    )?,
                ));
                !p.isolated()
            }
            None => true,
        };
        env.push((
            "HOU_GLOBAL_PACKAGES_ENABLED".into(),
            if global_packages_enabled { "1" } else { "0" }.into(),
        ));

        Ok(env)
    }

    pub fn launch_houdini<I, S>(
        &self,
        args: I,
        project: Option<&Project>,
        attach: bool,
    ) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let hou_executable = self.hfs.join("bin").join("houdini");
        let mut cmd = Command::new(hou_executable);
        cmd.args(args);
        cmd.envs(self.env(project)?);
        if let Some(p) = project {
            cmd.current_dir(&p.root);
        }
        if attach {
            cmd.stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .context(format!("Failed to run {:?}", cmd))?;
        } else {
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd.process_group(0);
            }
            cmd.spawn()
                .context(format!("Failed to spawn {:?}", cmd))?;
        }
        Ok(())
    }

    pub fn run(&self, mut cmd: Command, project: Option<&Project>) -> Result<ExitStatus> {
        cmd.envs(self.env(project)?)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        if let Some(p) = project {
            cmd.current_dir(&p.root);
        }
        cmd.status().context(format!("Failed to run {:?}", cmd))
    }
}
