use std::path::PathBuf;

pub struct GitHubSource {
    pub url: String,
    pub hash: String,
}

pub enum PackageSource {
    GitHub(GitHubSource),
    Local(PathBuf),
}

pub struct Package {
    pub name: String,
    pub source: PackageSource,
}

pub struct PackageCache {
    pub packages: Vec<Package>,
}

pub struct InstalledPackage {
    pub package: Package,
    pub path: PathBuf,
}

pub enum Scope {
    Global,
    Project(PathBuf),
}
pub struct InstalledPackages {
    pub scope: Scope,
    pub packages: Vec<InstalledPackage>,
}
