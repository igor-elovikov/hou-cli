use anyhow::{Context, Result, anyhow, bail};
use std::path::PathBuf;

pub enum InstallSource {
    WebGit { url: String, version: String },
    Folder { path: PathBuf },
}

pub struct InstallSpec {
    pub source: InstallSource,
    pub name: Option<String>,
}

impl InstallSpec {
    pub fn parse(
        raw: &str,
        tag: Option<String>,
        latest: bool,
        name: Option<String>,
    ) -> Result<Self> {
        if tag.is_some() && latest {
            bail!("--tag and --latest are mutually exclusive");
        }

        if looks_like_url(raw) {
            let version = tag.unwrap_or_else(|| "latest".into());
            return Ok(Self {
                source: InstallSource::WebGit {
                    url: raw.to_string(),
                    version,
                },
                name,
            });
        }

        let path = PathBuf::from(expand_tilde(raw));
        let path = path
            .canonicalize()
            .with_context(|| format!("Source path not found: {}", path.display()))?;

        if !path.is_dir() {
            return Err(anyhow!("Source must be a URL or a directory: {}", path.display()));
        }

        if tag.is_some() || latest {
            bail!("--tag / --latest only apply to web git sources");
        }

        Ok(Self {
            source: InstallSource::Folder { path },
            name,
        })
    }
}

pub fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.starts_with("git://")
}

fn expand_tilde(raw: &str) -> String {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new() {
            return home.home_dir().join(stripped).to_string_lossy().into_owned();
        }
    }
    raw.to_string()
}
