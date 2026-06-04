use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xxhash_rust::xxh3::Xxh3;

const CHUNK: usize = 64 * 1024;

pub fn dir_digest(root: &Path) -> Result<String> {
    let mut entries: Vec<(PathBuf, PathBuf)> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let abs = e.path().to_path_buf();
            let rel = abs.strip_prefix(root).unwrap_or(&abs).to_path_buf();
            (rel, abs)
        })
        .collect();

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Xxh3::new();
    let mut buf = vec![0u8; CHUNK];
    for (rel, abs) in &entries {
        let rel_bytes = rel.to_string_lossy();
        hasher.update(rel_bytes.as_bytes());
        hasher.update(&[0u8]);
        let file = File::open(abs).with_context(|| format!("Failed to open {}", abs.display()))?;
        let mut reader = BufReader::new(file);
        loop {
            let n = reader
                .read(&mut buf)
                .with_context(|| format!("Failed to read {}", abs.display()))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        hasher.update(&[0u8]);
    }

    Ok(format!("{:032x}", hasher.digest128()))
}

pub fn short_url_hash(url: &str) -> String {
    let mut h = Xxh3::new();
    h.update(url.as_bytes());
    let v = h.digest();
    format!("{:016x}", v)[..8].to_string()
}
