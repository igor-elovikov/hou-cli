use crate::installations::HoudiniInstallation;
use crate::package::manifest::{Manifest, PipManifest};
use crate::project::{PROJECT_MARKER, Project};
use anyhow::{Context, Result, bail};
use console::style;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const PIP_PACKAGES_DIR: &str = "pip-packages";
const FORCED_SITE_PACKAGES: &str = "site-packages-forced";
const ENV_VALUE_PREFIX: &str = "$HPROJECT/pip-packages";

/// Prints interpreter version and install paths for the prefix in argv[1].
const LAYOUT_SCRIPT: &str = r#"
import json, os, sys, sysconfig
p = sys.argv[1]
get = getattr(sysconfig, "get_preferred_scheme", None)
scheme = get("prefix") if get else ("nt" if os.name == "nt" else "posix_prefix")
v = {"base": p, "platbase": p, "installed_base": p, "installed_platbase": p}
print(json.dumps({
    "version": "%d.%d" % sys.version_info[:2],
    "site": sysconfig.get_path("purelib"),
    "purelib": sysconfig.get_path("purelib", scheme, v),
    "platlib": sysconfig.get_path("platlib", scheme, v),
    "scripts": sysconfig.get_path("scripts", scheme, v),
}))
"#;

/// Interpreter version and paths for a `--prefix` install.
#[derive(Debug, Deserialize)]
struct PythonLayout {
    version: String,
    site: PathBuf,
    purelib: PathBuf,
    platlib: PathBuf,
    scripts: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PipSyncStatus {
    Ok(usize),
    Repaired(usize),
}

/// Project pip packages installed with `--prefix` into `<project>/pip-packages`.
pub struct Pip<'a> {
    project: &'a Project,
    python: PathBuf,
    layout: PythonLayout,
    manifest: Manifest,
}

impl<'a> Pip<'a> {
    pub fn open(houdini: &HoudiniInstallation, project: &'a Project) -> Result<Self> {
        let python = houdini.python()?;
        let prefix = project.root.join(PIP_PACKAGES_DIR);
        let layout = query_layout(&python, &prefix)?;
        log::debug!("Python {} layout: {:?}", layout.version, layout);
        let manifest = Manifest::load_from(&project.manifest_path)?;
        Ok(Self {
            project,
            python,
            layout,
            manifest,
        })
    }

    pub fn install(&mut self, specs: &[String]) -> Result<()> {
        let mut pip = self.manifest.pip.clone().unwrap_or_default();
        let names = specs
            .iter()
            .map(|s| requirement_name(s))
            .collect::<Result<BTreeSet<_>>>()?;
        pip.requirements
            .retain(|r| requirement_name(r).map_or(true, |n| !names.contains(&n)));
        pip.requirements.extend(specs.iter().cloned());

        let constraints = pins_except(&pip.lock, &names);
        let result = self.resolve(pip, &constraints);
        if constraints.is_empty() {
            return result;
        }
        result.context("Failed to install while keeping existing pins; try `hou pip update`")
    }

    pub fn uninstall(&mut self, names: &[String]) -> Result<()> {
        let mut pip = self.manifest.pip.clone().unwrap_or_default();
        for name in names {
            let name = requirement_name(name)?;
            let before = pip.requirements.len();
            pip.requirements
                .retain(|r| requirement_name(r).map_or(true, |n| n != name));
            if pip.requirements.len() == before {
                bail!("{name} is not a project pip requirement");
            }
        }

        if pip.requirements.is_empty() {
            return self.clear();
        }
        let constraints = pins_except(&pip.lock, &BTreeSet::new());
        self.resolve(pip, &constraints)
    }

    /// Re-resolves to the newest allowed versions; everything when `names` is empty.
    pub fn update(&mut self, names: &[String]) -> Result<()> {
        let Some(pip) = self.manifest.pip.clone() else {
            bail!("No pip packages in this project");
        };
        let names = names
            .iter()
            .map(|s| requirement_name(s))
            .collect::<Result<BTreeSet<_>>>()?;
        let locked: BTreeSet<_> = pip
            .lock
            .iter()
            .chain(&pip.requirements)
            .filter_map(|l| requirement_name(l).ok())
            .collect();
        if let Some(missing) = names.iter().find(|n| !locked.contains(*n)) {
            bail!("{missing} is not installed in this project");
        }

        let constraints = if names.is_empty() {
            Vec::new()
        } else {
            pins_except(&pip.lock, &names)
        };
        self.resolve(pip, &constraints)
    }

    /// Reinstalls from the lock when the prefix is missing, stale, or built for another Python.
    pub fn sync(&mut self) -> Result<Option<PipSyncStatus>> {
        let Some(mut pip) = self.manifest.pip.clone() else {
            return Ok(None);
        };

        let prefix = self.prefix();
        let up_to_date = pip.python == self.layout.version
            && same_packages(&self.freeze(&prefix)?, &pip.lock);

        let status = if up_to_date {
            PipSyncStatus::Ok(pip.lock.len())
        } else {
            log::warn!("Pip packages out of date, reinstalling from lock");
            let lockfile = lines_file(&pip.lock)?;
            let args: Vec<OsString> = vec!["-r".into(), lockfile.path().into()];
            self.build(&args)?;
            pip.python = self.layout.version.clone();
            self.manifest.pip = Some(pip.clone());
            self.manifest.save_to(&self.project.manifest_path)?;
            PipSyncStatus::Repaired(pip.lock.len())
        };

        self.write_project_env(true)?;
        Ok(Some(status))
    }

    fn prefix(&self) -> PathBuf {
        self.project.root.join(PIP_PACKAGES_DIR)
    }

    /// Builds a fresh prefix from `pip.requirements`, then stores the new lock.
    fn resolve(&mut self, mut pip: PipManifest, constraints: &[String]) -> Result<()> {
        let mut args: Vec<OsString> = pip.requirements.iter().map(Into::into).collect();
        let constraints_file = lines_file(constraints)?;
        if !constraints.is_empty() {
            args.extend(["-c".into(), constraints_file.path().into()]);
        }

        pip.lock = self.build(&args)?;
        pip.python = self.layout.version.clone();
        println!("Locked {} pip package(s)", pip.lock.len());
        self.warn_forced(&pip.lock);

        self.manifest.pip = Some(pip);
        self.manifest.save_to(&self.project.manifest_path)?;
        self.write_project_env(true)
    }

    /// Installs into a staging prefix and swaps it in on success; returns the freeze of it.
    fn build(&self, install_args: &[OsString]) -> Result<Vec<String>> {
        let prefix = self.prefix();
        let staging = tempfile::Builder::new()
            .prefix(".pip-packages-")
            .tempdir_in(&self.project.root)
            .context("Failed to create pip staging directory")?;

        let mut args: Vec<OsString> = vec![
            "install".into(),
            "--prefix".into(),
            staging.path().into(),
            "--no-warn-script-location".into(),
        ];
        args.extend(install_args.iter().cloned());
        self.run_pip(&args)?;

        let lock = self.freeze(staging.path())?;
        replace_dir(staging.path(), &prefix)?;
        Ok(lock)
    }

    fn clear(&mut self) -> Result<()> {
        let prefix = self.prefix();
        if prefix.exists() {
            fs::remove_dir_all(&prefix)
                .with_context(|| format!("Failed to remove {}", prefix.display()))?;
        }
        self.manifest.pip = None;
        self.manifest.save_to(&self.project.manifest_path)?;
        println!("Removed all pip packages");
        self.write_project_env(false)
    }

    /// Warns about locked packages that Houdini's forced site-packages shadow at runtime.
    fn warn_forced(&self, lock: &[String]) {
        let forced_dir = self.layout.site.with_file_name(FORCED_SITE_PACKAGES);
        let Ok(entries) = fs::read_dir(&forced_dir) else {
            return;
        };
        let forced: BTreeSet<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let stem = name
                    .strip_suffix(".dist-info")
                    .or_else(|| name.strip_suffix(".egg-info"))?;
                requirement_name(stem.split('-').next()?).ok()
            })
            .collect();
        for line in lock {
            if requirement_name(line).is_ok_and(|n| forced.contains(&n)) {
                eprintln!(
                    "{} {} is shadowed at runtime by Houdini's {}",
                    style("warning:").yellow().bold(),
                    style(line).yellow(),
                    forced_dir.display(),
                );
            }
        }
    }

    /// Bundled interpreter isolated from the user's Python environment.
    fn python_cmd(&self) -> Command {
        let mut cmd = Command::new(&self.python);
        cmd.env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .env("PYTHONNOUSERSITE", "1");
        cmd
    }

    /// Runs pip from a throwaway venv over Houdini's site-packages (forced ones included):
    /// bundled packages count as installed but lie outside the environment, so pip never
    /// uninstalls them from the Houdini install.
    fn run_pip(&self, args: &[OsString]) -> Result<()> {
        let venv = tempfile::Builder::new()
            .prefix("hou-pip-venv-")
            .tempdir()
            .context("Failed to create temp venv directory")?;
        let status = self
            .python_cmd()
            .args(["-m", "venv", "--system-site-packages", "--without-pip"])
            .arg(venv.path())
            .status()
            .with_context(|| format!("Failed to run {}", self.python.display()))?;
        if !status.success() {
            bail!("Failed to create venv ({status})");
        }
        let venv_python = if cfg!(windows) {
            venv.path().join("Scripts").join("python.exe")
        } else {
            venv.path().join("bin").join("python")
        };

        let forced = self.layout.site.with_file_name(FORCED_SITE_PACKAGES);
        let mut cmd = Command::new(&venv_python);
        cmd.env_remove("PYTHONHOME")
            .env("PYTHONNOUSERSITE", "1")
            .args(["-m", "pip", "--disable-pip-version-check"])
            .args(args)
            .env("PIP_USER", "0")
            .env("PIP_REQUIRE_VIRTUALENV", "0");
        if forced.is_dir() {
            cmd.env("PYTHONPATH", &forced);
        } else {
            cmd.env_remove("PYTHONPATH");
        }
        log::debug!("Running {:?}", cmd);
        let status = cmd
            .status()
            .with_context(|| format!("Failed to run {}", venv_python.display()))?;
        if !status.success() {
            bail!("pip failed ({status})");
        }
        Ok(())
    }

    /// `pip freeze` of the site-packages dirs under `prefix`.
    fn freeze(&self, prefix: &Path) -> Result<Vec<String>> {
        let mut cmd = self.python_cmd();
        cmd.args(["-m", "pip", "--disable-pip-version-check", "freeze", "--all"]);
        let mut any = false;
        for lib in self.site_dirs() {
            let dir = prefix.join(lib);
            if dir.is_dir() {
                cmd.arg("--path").arg(dir);
                any = true;
            }
        }
        if !any {
            return Ok(Vec::new());
        }

        let output = cmd
            .output()
            .with_context(|| format!("Failed to run {}", self.python.display()))?;
        if !output.status.success() {
            bail!(
                "pip freeze failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect())
    }

    /// Site-packages dirs relative to the prefix.
    fn site_dirs(&self) -> Vec<PathBuf> {
        let prefix = self.prefix();
        let mut dirs: Vec<PathBuf> = [&self.layout.purelib, &self.layout.platlib]
            .iter()
            .filter_map(|p| p.strip_prefix(&prefix).ok().map(Path::to_path_buf))
            .collect();
        dirs.dedup();
        dirs
    }

    /// Syncs pip env entries of the project package file (`hproject.json`).
    fn write_project_env(&self, enabled: bool) -> Result<()> {
        let marker = self.project.root.join(PROJECT_MARKER);
        let text = fs::read_to_string(&marker)
            .with_context(|| format!("Failed to read {}", marker.display()))?;
        let mut doc: Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse {}", marker.display()))?;
        let obj = doc
            .as_object_mut()
            .with_context(|| format!("{} is not a JSON object", marker.display()))?;
        let env = obj
            .entry("env")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .with_context(|| format!("`env` in {} is not an array", marker.display()))?;

        let before = env.clone();
        env.retain(|e| !is_pip_env_entry(e));
        if enabled {
            let prefix = self.prefix();
            for dir in self.site_dirs() {
                if prefix.join(&dir).is_dir() {
                    env.push(prepend_entry("PYTHONPATH", &dir));
                }
            }
            if let Ok(scripts) = self.layout.scripts.strip_prefix(&prefix)
                && self.layout.scripts.is_dir()
            {
                env.push(prepend_entry("PATH", scripts));
            }
        }
        if *env == before {
            return Ok(());
        }

        let body = serde_json::to_string_pretty(&doc)?;
        fs::write(&marker, format!("{body}\n"))
            .with_context(|| format!("Failed to write {}", marker.display()))?;
        log::info!("Updated pip env in {}", marker.display());
        Ok(())
    }
}

fn query_layout(python: &Path, prefix: &Path) -> Result<PythonLayout> {
    let output = Command::new(python)
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .env("PYTHONNOUSERSITE", "1")
        .arg("-c")
        .arg(LAYOUT_SCRIPT)
        .arg(prefix)
        .output()
        .with_context(|| format!("Failed to run {}", python.display()))?;
    if !output.status.success() {
        bail!(
            "Failed to query Python layout: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    serde_json::from_slice(&output.stdout).context("Failed to parse Python layout")
}

/// `{"VAR": {"value": "$HPROJECT/pip-packages/<rel>", "method": "prepend"}}`.
fn prepend_entry(var: &str, rel: &Path) -> Value {
    let rel = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    json!({ var: { "value": format!("{ENV_VALUE_PREFIX}/{rel}"), "method": "prepend" } })
}

fn is_pip_env_entry(entry: &Value) -> bool {
    let Some(obj) = entry.as_object() else {
        return false;
    };
    obj.values().any(|v| {
        let value = v.get("value").unwrap_or(v);
        value
            .as_str()
            .is_some_and(|s| s.starts_with(ENV_VALUE_PREFIX))
    })
}

/// Canonical (PEP 503) project name of a requirement spec or freeze line.
fn requirement_name(spec: &str) -> Result<String> {
    let raw: String = spec
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    if raw.is_empty() {
        bail!("Unsupported requirement `{spec}`: expected a package name");
    }
    let mut name = String::with_capacity(raw.len());
    for c in raw.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !name.ends_with('-') {
                name.push('-');
            }
        } else {
            name.push(c.to_ascii_lowercase());
        }
    }
    Ok(name)
}

/// Lock pins usable as pip constraints, minus `names`.
fn pins_except(lock: &[String], names: &BTreeSet<String>) -> Vec<String> {
    lock.iter()
        .filter(|l| l.contains("==") && !l.contains(" @ "))
        .filter(|l| requirement_name(l).is_ok_and(|n| !names.contains(&n)))
        .cloned()
        .collect()
}

fn same_packages(a: &[String], b: &[String]) -> bool {
    let norm = |v: &[String]| v.iter().map(|l| l.to_ascii_lowercase()).collect::<BTreeSet<_>>();
    norm(a) == norm(b)
}

fn lines_file(lines: &[String]) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new().context("Failed to create temp file")?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    file.flush()?;
    Ok(file)
}

/// Moves `src` over `dst`, keeping `dst` intact until the move succeeds.
fn replace_dir(src: &Path, dst: &Path) -> Result<()> {
    let backup = dst.with_file_name(format!(".{PIP_PACKAGES_DIR}-old"));
    if backup.exists() {
        fs::remove_dir_all(&backup)
            .with_context(|| format!("Failed to remove {}", backup.display()))?;
    }
    if dst.exists() {
        fs::rename(dst, &backup)
            .with_context(|| format!("Failed to move {}", dst.display()))?;
    }
    if let Err(e) = fs::rename(src, dst) {
        if backup.exists() {
            let _ = fs::rename(&backup, dst);
        }
        return Err(e).with_context(|| format!("Failed to move pip packages to {}", dst.display()));
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)
            .with_context(|| format!("Failed to remove {}", backup.display()))?;
    }
    Ok(())
}
