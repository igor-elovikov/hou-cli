use crate::installations::HoudiniInstallation;
use crate::project::Project;
use crate::utils::{env_paths_added, env_paths_prepended};
use anyhow::{Context, Result};
use is_executable::IsExecutable;
use semver::Version;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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
            path,
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
    fn user_prefs_dir(version: &Version) -> anyhow::Result<PathBuf> {
        let dirs =
            directories::BaseDirs::new().context("Failed to get user preference directory")?;
        let pref = dirs.preference_dir();

        let houdini_prefs = pref
            .join("houdini")
            .join(format!("{}.{}", version.major, version.minor));

        Ok(houdini_prefs)
    }

    #[cfg(target_os = "windows")]
    fn user_prefs_dir(version: &Version) -> Result<PathBuf> {
        let user_dirs =
            directories::UserDirs::new().context("Failed to get user preference directory")?;
        let pref = user_dirs
            .document_dir()
            .context("Failed to get Documents directory")?;

        let houdini_prefs = pref.join(format!("houdini{}.{}", version.major, version.minor));

        Ok(houdini_prefs)
    }

    fn env(&self, project: Option<&Project>) -> Result<Vec<(OsString, OsString)>> {
        let bin_path = self.hfs.join("bin");
        let hb = self.hfs.join("bin");
        let hdso = self.hfs.join("..").join("Libraries");
        let hh = self.hfs.join("houdini");
        let sbin_path = hh.join("sbin");
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

    pub fn launch<I, S>(&self, args: I, project: Option<&Project>, attach: bool) -> Result<()>
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
            cmd.spawn().context(format!("Failed to spawn {:?}", cmd))?;
        }
        Ok(())
    }

    pub fn run(&self, mut cmd: Command, project: Option<&Project>) -> Result<ExitStatus> {
        cmd.envs(self.env(project)?)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        cmd.status().context(format!("Failed to run {:?}", cmd))
    }

    /// Bundled Python interpreter (plain python, not hython).
    pub fn python(&self) -> Result<PathBuf> {
        self.python_homes()
            .iter()
            .find_map(|home| python_executable(home))
            .with_context(|| {
                format!(
                    "No bundled Python interpreter found for Houdini {}",
                    self.version
                )
            })
    }

    /// Version of the bundled Python interpreter.
    pub fn python_version(&self) -> Result<Version> {
        let python = self.python()?;
        let output = Command::new(&python)
            .arg("--version")
            .output()
            .with_context(|| format!("Failed to run {}", python.display()))?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        text.split_whitespace()
            .find_map(parse_python_version)
            .with_context(|| format!("Failed to parse Python version from {:?}", text.trim()))
    }

    /// Candidate Python home dirs, preferred first.
    fn python_homes(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            let versions = self
                .path
                .join("Frameworks")
                .join("Python.framework")
                .join("Versions");
            let mut homes = vec![versions.join("Current")];
            homes.extend(versioned_dirs(&versions, ""));
            homes
        } else {
            let mut homes = vec![self.hfs.join("python")];
            homes.extend(versioned_dirs(&self.hfs, "python"));
            homes
        }
    }
}

/// Subdirs of `dir` named `<prefix><version>` (`3.13`, `python311`), newest first.
fn versioned_dirs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<((u64, u64), PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let key = python_major_minor(name.strip_prefix(prefix)?)?;
            Some((key, e.path()))
        })
        .collect();
    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    dirs.into_iter().map(|(_, p)| p).collect()
}

/// Major/minor from `3.13` or `313` (single digit major).
fn python_major_minor(s: &str) -> Option<(u64, u64)> {
    let (major, minor) = match s.split_once('.') {
        Some(parts) => parts,
        None if s.len() > 1 && s.is_ascii() => s.split_at(1),
        None => return None,
    };
    Some((major.parse().ok()?, minor.parse().ok()?))
}

/// Interpreter inside a Python home, `None` if absent.
fn python_executable(home: &Path) -> Option<PathBuf> {
    let (dir, ext) = if cfg!(windows) {
        (home.to_path_buf(), ".exe")
    } else {
        (home.join("bin"), "")
    };

    ["python3", "python"]
        .iter()
        .map(|name| dir.join(format!("{name}{ext}")))
        .find(|p| p.is_executable())
        .or_else(|| {
            // Fall back to the newest `python3.N` binary.
            let mut found: Vec<((u64, u64), PathBuf)> = std::fs::read_dir(&dir)
                .ok()?
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let version = name.strip_prefix("python")?.strip_suffix(ext)?;
                    let key = python_major_minor(version).filter(|_| version.contains('.'))?;
                    Some((key, e.path()))
                })
                .filter(|(_, p)| p.is_executable())
                .collect();
            found.sort_by(|a, b| b.0.cmp(&a.0));
            found.into_iter().next().map(|(_, p)| p)
        })
}

/// Lenient parse of `3.13.10` / `3.14.0rc1` into a semver version.
fn parse_python_version(s: &str) -> Option<Version> {
    let mut parts = s.split('.').map(|p| {
        let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u64>().ok()
    });
    let major = parts.next()??;
    let minor = parts.next()??;
    let patch = parts.next().flatten().unwrap_or(0);
    Some(Version::new(major, minor, patch))
}
