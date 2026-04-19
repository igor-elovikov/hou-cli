use anyhow::{Context, Result, anyhow};
use gix::progress::Discard;
use gix::remote::fetch::Shallow;
use indicatif::{ProgressBar, ProgressStyle};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::atomic::AtomicBool;
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

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} [{elapsed_precise}] {msg}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    let display_ref = ref_name.unwrap_or("HEAD");
    pb.set_message(format!("Fetching {url} @ {display_ref}"));

    let interrupt = AtomicBool::new(false);

    let mut prep = gix::prepare_clone(url, dest)
        .with_context(|| {
            pb.finish_and_clear();
            format!("Failed to initiate clone of {url}")
        })?
        .with_shallow(Shallow::DepthAtRemote(NonZeroU32::new(1).unwrap()));

    if let Some(name) = ref_name {
        prep = prep
            .with_ref_name(Some(name))
            .with_context(|| {
                pb.finish_and_clear();
                format!("Invalid ref name: {name}")
            })?;
    }

    let fetch_result = prep.fetch_then_checkout(Discard, &interrupt);
    let (mut checkout, _outcome) = match fetch_result {
        Ok(v) => v,
        Err(e) => {
            pb.finish_and_clear();
            return Err(anyhow::Error::from(e).context(format!("Failed to fetch from {url}")));
        }
    };

    pb.set_message(format!("Checking out {display_ref}"));

    let checkout_result = checkout.main_worktree(Discard, &interrupt);
    let (repo, _co) = match checkout_result {
        Ok(v) => v,
        Err(e) => {
            pb.finish_and_clear();
            return Err(anyhow::Error::from(e)
                .context(format!("Failed to check out worktree at {}", dest.display())));
        }
    };

    let commit = repo
        .head_id()
        .context("Failed to resolve HEAD after clone")?
        .to_string();
    log::info!("Fetched {url} @ {commit}");

    drop(repo);

    let git_dir = dest.join(".git");
    if git_dir.exists() {
        pb.set_message("Stripping .git");
        log::debug!("Stripping {}", git_dir.display());
        std::fs::remove_dir_all(&git_dir)
            .with_context(|| format!("Failed to strip .git at {}", git_dir.display()))?;
    }

    pb.finish_with_message(format!("Fetched {} @ {}", short_sha(&commit), display_ref));
    Ok(commit)
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(8)]
}

pub fn open_head_commit(path: &Path) -> Result<String> {
    let repo = gix::open(path)
        .with_context(|| format!("Failed to open git repo at {}", path.display()))?;
    let id = repo.head_id().context("Failed to resolve HEAD")?;
    Ok(id.to_string())
}

pub fn list_remote_tags(url: &str) -> Result<Vec<String>> {
    log::debug!("ls-refs {url}");
    let tmp = tempfile::tempdir().context("Failed to create scratch dir for ls-refs")?;
    let repo = gix::init_bare(tmp.path())
        .with_context(|| format!("Failed to init scratch repo at {}", tmp.path().display()))?;
    let remote = repo
        .remote_at(url)
        .with_context(|| format!("Invalid remote URL: {url}"))?;
    let connection = remote
        .connect(gix::remote::Direction::Fetch)
        .with_context(|| format!("Failed to connect to {url}"))?;
    let (ref_map, _handshake) = connection
        .ref_map(Discard, gix::remote::ref_map::Options::default())
        .with_context(|| format!("Failed to ls-refs on {url}"))?;

    let prefix = b"refs/tags/";
    let mut tags = Vec::new();
    for r in &ref_map.remote_refs {
        let (full, _id, _peeled) = r.unpack();
        if full.starts_with(prefix) {
            let name = std::str::from_utf8(&full[prefix.len()..])
                .map_err(|e| anyhow!("Non-utf8 tag name: {e}"))?;
            if !name.ends_with("^{}") {
                tags.push(name.to_string());
            }
        }
    }
    log::debug!("Found {} remote tag(s) on {url}", tags.len());
    Ok(tags)
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
