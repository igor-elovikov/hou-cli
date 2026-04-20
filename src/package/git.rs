use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

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

    let pb = spinner(format!(
        "Cloning {url} @ {}",
        ref_name.unwrap_or("HEAD")
    ));

    let mut cmd = Command::new("git");
    cmd.arg("clone")
        .arg("--depth=1")
        .arg("--single-branch")
        .arg("--quiet");
    if let Some(name) = ref_name {
        cmd.arg("--branch").arg(name);
    }
    cmd.arg(url).arg(dest);

    let out = cmd
        .output()
        .context("Failed to spawn `git clone` — is git on PATH?")?;
    if !out.status.success() {
        pb.finish_and_clear();
        bail!(
            "git clone failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let commit = head_commit(dest)?;
    log::info!("Fetched {url} @ {commit}");

    pb.finish_with_message(format!(
        "Cloned {} @ {}",
        short_sha(&commit),
        ref_name.unwrap_or("HEAD")
    ));
    Ok(commit)
}

pub fn fetch_update(dest: &Path, ref_name: Option<&str>) -> Result<String> {
    log::info!(
        "Shallow-fetching update in {} (ref={})",
        dest.display(),
        ref_name.unwrap_or("HEAD")
    );

    let pb = spinner(format!("Fetching {}", ref_name.unwrap_or("HEAD")));

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dest)
        .arg("fetch")
        .arg("--depth=1")
        .arg("--quiet")
        .arg("origin");
    if let Some(name) = ref_name {
        cmd.arg(name);
    }
    run(cmd, &pb, "git fetch")?;

    let reset_target = if ref_name.is_some() {
        "FETCH_HEAD"
    } else {
        "FETCH_HEAD"
    };
    pb.set_message("Resetting worktree");
    let mut reset = Command::new("git");
    reset
        .arg("-C")
        .arg(dest)
        .args(["reset", "--hard", "--quiet", reset_target]);
    run(reset, &pb, "git reset")?;

    pb.set_message("Cleaning untracked files");
    let mut clean = Command::new("git");
    clean
        .arg("-C")
        .arg(dest)
        .args(["clean", "-fdxq", "--exclude=!.git"]);
    run(clean, &pb, "git clean")?;

    let commit = head_commit(dest)?;
    pb.finish_with_message(format!(
        "Updated to {} @ {}",
        short_sha(&commit),
        ref_name.unwrap_or("HEAD")
    ));
    Ok(commit)
}

pub fn list_remote_tags(url: &str) -> Result<Vec<String>> {
    log::debug!("ls-remote --tags {url}");
    let out = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .output()
        .context("Failed to spawn `git ls-remote`")?;
    if !out.status.success() {
        bail!(
            "git ls-remote failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
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
    check_status(&out, "git rev-parse HEAD")?;
    let sha = String::from_utf8(out.stdout)
        .context("Non-utf8 output from git rev-parse")?
        .trim()
        .to_string();
    Ok(sha)
}

fn run(mut cmd: Command, pb: &ProgressBar, label: &str) -> Result<()> {
    let out = cmd
        .output()
        .with_context(|| format!("Failed to spawn `{label}`"))?;
    if !out.status.success() {
        pb.finish_and_clear();
        bail!(
            "{label} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn check_status(out: &Output, label: &str) -> Result<()> {
    if !out.status.success() {
        bail!(
            "{label} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn spinner(message: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} [{elapsed_precise}] {msg}")
            .expect("valid template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message);
    pb
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
