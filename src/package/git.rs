use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn clone_at(url: &str, dest: &Path, ref_name: Option<&str>) -> Result<String> {
    log::info!(
        "Shallow-cloning {url} (ref={}) into {}",
        ref_name.unwrap_or("HEAD"),
        dest.display()
    );

    if dest.exists() {
        log::debug!("Clearing existing {}", dest.display());
        std::fs::remove_dir_all(dest)
            .with_context(|| format!("Failed to clear {}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let mut cmd = Command::new("git");
    // Tag checkouts print the long detached-HEAD advice otherwise.
    cmd.args(["-c", "advice.detachedHead=false", "clone"])
        .arg("--depth=1")
        .arg("--single-branch");
    if let Some(name) = ref_name {
        cmd.arg("--branch").arg(name);
    }
    cmd.arg(url).arg(dest);
    run_interactive(cmd, "git clone")?;

    let commit = head_commit(dest)?;
    log::info!("Fetched {url} @ {commit}");

    println!(
        "Cloned {} @ {}",
        short_sha(&commit),
        ref_name.unwrap_or("HEAD")
    );
    Ok(commit)
}

pub fn fetch_update(dest: &Path, ref_name: Option<&str>) -> Result<String> {
    log::info!(
        "Shallow-fetching update in {} (ref={})",
        dest.display(),
        ref_name.unwrap_or("HEAD")
    );

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dest)
        .arg("fetch")
        .arg("--depth=1")
        .arg("origin");
    if let Some(name) = ref_name {
        cmd.arg(name);
    }
    run_interactive(cmd, "git fetch")?;

    let mut reset = Command::new("git");
    reset
        .arg("-C")
        .arg(dest)
        .args(["reset", "--hard", "--quiet", "FETCH_HEAD"]);
    run_local(reset, "git reset")?;

    let mut clean = Command::new("git");
    clean
        .arg("-C")
        .arg(dest)
        .args(["clean", "-fdxq", "--exclude=!.git"]);
    run_local(clean, "git clean")?;

    let commit = head_commit(dest)?;
    println!(
        "Updated to {} @ {}",
        short_sha(&commit),
        ref_name.unwrap_or("HEAD")
    );
    Ok(commit)
}

pub fn list_remote_tags(url: &str) -> Result<Vec<String>> {
    log::debug!("ls-remote --tags {url}");
    let out = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn `git ls-remote` — is git on PATH?")?
        .wait_with_output()
        .context("Failed to wait for `git ls-remote`")?;
    if !out.status.success() {
        bail!("git ls-remote failed ({})", out.status);
    }

    let stdout = String::from_utf8(out.stdout).context("Non-utf8 output from git ls-remote")?;
    let mut tags = Vec::new();
    for line in stdout.lines() {
        let Some((_sha, full)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = full.strip_prefix("refs/tags/") else {
            continue;
        };
        if name.ends_with("^{}") {
            continue;
        }
        tags.push(name.to_string());
    }
    log::debug!("Found {} remote tag(s) on {url}", tags.len());
    Ok(tags)
}

fn head_commit(dest: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dest)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("Failed to spawn `git rev-parse HEAD`")?;
    if !out.status.success() {
        bail!(
            "git rev-parse failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8(out.stdout)
        .context("Non-utf8 output from git rev-parse")?
        .trim()
        .to_string())
}

/// Run a git command that may prompt for credentials or print progress.
/// All three std streams inherit from the parent so credential helpers and
/// tty prompts work.
fn run_interactive(mut cmd: Command, label: &str) -> Result<()> {
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd
        .status()
        .with_context(|| format!("Failed to spawn `{label}` — is git on PATH?"))?;
    if !status.success() {
        bail!("{label} failed ({status})");
    }
    Ok(())
}

/// Run a local git command that never needs the network and shouldn't prompt.
fn run_local(mut cmd: Command, label: &str) -> Result<()> {
    let out = cmd
        .output()
        .with_context(|| format!("Failed to spawn `{label}`"))?;
    if !out.status.success() {
        bail!(
            "{label} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(8)]
}

pub fn ref_kind_from_version(version: &str) -> RefKind<'_> {
    if version == "latest" {
        RefKind::Latest
    } else {
        RefKind::Tag(version)
    }
}

pub enum RefKind<'a> {
    Latest,
    Tag(&'a str),
}

impl<'a> RefKind<'a> {
    pub fn as_ref_name(&self) -> Option<&'a str> {
        match self {
            RefKind::Latest => None,
            RefKind::Tag(t) => Some(*t),
        }
    }
}
