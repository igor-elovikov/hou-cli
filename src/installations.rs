use anyhow::Result;
use semver::Version;
use std::path::{Path, PathBuf};
#[derive(Debug)]
pub struct Installation {
    pub path: PathBuf,
    pub version: Version,
    pub ready: bool,
}

#[derive(Debug)]
pub struct HoudiniInstallation {
    pub hfs: PathBuf,
    /// Install root as registered by the launcher.
    pub path: PathBuf,
    pub version: Version,
    pub user_prefs_dir: PathBuf,
    pub ready: bool,
}

#[derive(Debug)]
pub enum InstalledProduct {
    Houdini(HoudiniInstallation),
    HServer(Installation),
    LicenseServer(Installation),
    HQueueServer(Installation),
    HQueueClient(Installation),
}

impl InstalledProduct {
    pub fn kind(&self) -> &'static str {
        match self {
            InstalledProduct::Houdini(_) => "houdini",
            InstalledProduct::HServer(_) => "hserver",
            InstalledProduct::LicenseServer(_) => "license-server",
            InstalledProduct::HQueueServer(_) => "hqueue-server",
            InstalledProduct::HQueueClient(_) => "hqueue-client",
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            InstalledProduct::Houdini(h) => &h.path,
            InstalledProduct::HServer(i)
            | InstalledProduct::LicenseServer(i)
            | InstalledProduct::HQueueServer(i)
            | InstalledProduct::HQueueClient(i) => &i.path,
        }
    }

    pub fn version(&self) -> &Version {
        match self {
            InstalledProduct::Houdini(h) => &h.version,
            InstalledProduct::HServer(i)
            | InstalledProduct::LicenseServer(i)
            | InstalledProduct::HQueueServer(i)
            | InstalledProduct::HQueueClient(i) => &i.version,
        }
    }
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

