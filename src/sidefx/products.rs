use anyhow::{Result, anyhow};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Product {
    Houdini(Houdini),
    HoudiniLauncher(HoudiniLauncher),
    LauncherIso(LauncherIso),
    Docker,
    SidefxLabs,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Houdini {
    Default,
    Py3,
    Py37,
    Py2,
    Qt6,
    Gcc9,
    Gcc9Py39,
    Gcc9Py310,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum HoudiniLauncher {
    Default,
    Py3,
    Py37,
    Gcc9,
    Gcc9Py310,
    Qt6,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum LauncherIso {
    Default,
    Py3,
    Py37,
    Py310,
    Py2,
    Qt5,
    Qt6,
    Gcc9,
    Gcc9Py39,
    Gcc9Py310,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Platform {
    Win64,
    Macos,
    MacosxArm64,
    Linux,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Release {
    Gold,
    Development,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Status {
    Good,
    Bad,
}

impl Product {
    pub(super) fn as_api_str(&self) -> &'static str {
        match self {
            Product::Docker => "docker",
            Product::SidefxLabs => "sidefxlabs",
            Product::Houdini(v) => match v {
                Houdini::Default => "houdini",
                Houdini::Py3 => "houdini-py3",
                Houdini::Py37 => "houdini-py37",
                Houdini::Py2 => "houdini-py2",
                Houdini::Qt6 => "houdini-qt6",
                Houdini::Gcc9 => "houdini-gcc9",
                Houdini::Gcc9Py39 => "houdini-gcc9-py39",
                Houdini::Gcc9Py310 => "houdini-gcc9-py310",
            },
            Product::HoudiniLauncher(v) => match v {
                HoudiniLauncher::Default => "houdini-launcher",
                HoudiniLauncher::Py3 => "houdini-launcher-py3",
                HoudiniLauncher::Py37 => "houdini-launcher-py37",
                HoudiniLauncher::Gcc9 => "houdini-launcher-gcc9",
                HoudiniLauncher::Gcc9Py310 => "houdini-launcher-gcc9-py310",
                HoudiniLauncher::Qt6 => "houdini-launcher-qt6",
            },
            Product::LauncherIso(v) => match v {
                LauncherIso::Default => "launcher-iso",
                LauncherIso::Py3 => "launcher-iso-py3",
                LauncherIso::Py37 => "launcher-iso-py37",
                LauncherIso::Py310 => "launcher-iso-py310",
                LauncherIso::Py2 => "launcher-iso-py2",
                LauncherIso::Qt5 => "launcher-iso-qt5",
                LauncherIso::Qt6 => "launcher-iso-qt6",
                LauncherIso::Gcc9 => "launcher-iso-gcc9",
                LauncherIso::Gcc9Py39 => "launcher-iso-gcc9-py39",
                LauncherIso::Gcc9Py310 => "launcher-iso-gcc9-py310",
            },
        }
    }
}

impl FromStr for Product {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "houdini" => Product::Houdini(Houdini::Default),
            "houdini-py3" => Product::Houdini(Houdini::Py3),
            "houdini-py37" => Product::Houdini(Houdini::Py37),
            "houdini-py2" => Product::Houdini(Houdini::Py2),
            "houdini-qt6" => Product::Houdini(Houdini::Qt6),
            "houdini-gcc9" => Product::Houdini(Houdini::Gcc9),
            "houdini-gcc9-py39" => Product::Houdini(Houdini::Gcc9Py39),
            "houdini-gcc9-py310" => Product::Houdini(Houdini::Gcc9Py310),
            "houdini-launcher" => Product::HoudiniLauncher(HoudiniLauncher::Default),
            "houdini-launcher-py3" => Product::HoudiniLauncher(HoudiniLauncher::Py3),
            "houdini-launcher-py37" => Product::HoudiniLauncher(HoudiniLauncher::Py37),
            "houdini-launcher-gcc9" => Product::HoudiniLauncher(HoudiniLauncher::Gcc9),
            "houdini-launcher-gcc9-py310" => Product::HoudiniLauncher(HoudiniLauncher::Gcc9Py310),
            "houdini-launcher-qt6" => Product::HoudiniLauncher(HoudiniLauncher::Qt6),
            "launcher-iso" => Product::LauncherIso(LauncherIso::Default),
            "launcher-iso-py3" => Product::LauncherIso(LauncherIso::Py3),
            "launcher-iso-py37" => Product::LauncherIso(LauncherIso::Py37),
            "launcher-iso-py310" => Product::LauncherIso(LauncherIso::Py310),
            "launcher-iso-py2" => Product::LauncherIso(LauncherIso::Py2),
            "launcher-iso-qt5" => Product::LauncherIso(LauncherIso::Qt5),
            "launcher-iso-qt6" => Product::LauncherIso(LauncherIso::Qt6),
            "launcher-iso-gcc9" => Product::LauncherIso(LauncherIso::Gcc9),
            "launcher-iso-gcc9-py39" => Product::LauncherIso(LauncherIso::Gcc9Py39),
            "launcher-iso-gcc9-py310" => Product::LauncherIso(LauncherIso::Gcc9Py310),
            "docker" => Product::Docker,
            "sidefxlabs" => Product::SidefxLabs,
            other => return Err(anyhow!("unknown product: {other}")),
        })
    }
}

impl Platform {
    pub(super) fn as_api_str(&self) -> &'static str {
        match self {
            Platform::Win64 => "win64",
            Platform::Macos => "macos",
            Platform::MacosxArm64 => "macosx_arm64",
            Platform::Linux => "linux",
        }
    }
}

impl FromStr for Platform {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.starts_with("linux") {
            Ok(Platform::Linux)
        } else if s.starts_with("win") {
            Ok(Platform::Win64)
        } else if s.starts_with("macosx_arm64") || s.starts_with("macos_arm64") {
            Ok(Platform::MacosxArm64)
        } else if s.starts_with("macos") {
            Ok(Platform::Macos)
        } else {
            Err(anyhow!("unknown platform: {s}"))
        }
    }
}

impl FromStr for Release {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "gold" => Ok(Release::Gold),
            "development" | "devel" => Ok(Release::Development),
            other => Err(anyhow!("unknown release: {other}")),
        }
    }
}

impl FromStr for Status {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "good" => Ok(Status::Good),
            "bad" => Ok(Status::Bad),
            other => Err(anyhow!("unknown status: {other}")),
        }
    }
}
