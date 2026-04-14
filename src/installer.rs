use crate::products::{HoudiniInstallation, Installation, Product};
use anyhow::{Context, Result, bail};
use is_executable::IsExecutable;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct Installer {
    installer_exe: PathBuf,
}

pub enum InstallerCommand {
    Install,
    Uninstall,
    List,
}

impl InstallerCommand {
    pub fn args(&self) -> Vec<String> {
        match self {
            InstallerCommand::Install => vec!["install".to_owned()],
            InstallerCommand::Uninstall => vec!["uninstall".to_owned()],
            InstallerCommand::List => vec!["list".to_owned()],
        }
    }
}

impl Installer {
    pub fn discover(data_path: &Path) -> Result<Self> {
        let candidates = Self::candidate_paths(data_path);

        for path in &candidates {
            if path.is_executable() {
                return Ok(Self {
                    installer_exe: path.clone(),
                });
            }
        }

        bail!(
            "No houdini_installer found. Searched:\n{}",
            candidates
                .iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    pub fn run(&self, command: InstallerCommand) -> Result<String> {
        let mut cmd = Command::new(self.installer_exe.clone().into_os_string());

        let stdout = cmd.args(&command.args()).output()?.stdout;

        String::from_utf8(stdout).context("Failed to parse stdout")
    }

    pub fn products(&self) -> Result<Vec<Product>> {
        let list_result = self.run(InstallerCommand::List)?;

        let mut lines = list_result.lines();
        let header = lines.next().context("Empty installer output")?;

        let comp_col = header
            .find("Component")
            .context("Missing Component column")?;
        let ver_col = header.find("Version").context("Missing Version column")?;
        let ready_col = header.find("Ready?").context("Missing Ready? column")?;

        let mut products = Vec::new();

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let path = line[..comp_col].trim();
            let component = line[comp_col..ver_col].trim();
            let version = line[ver_col..ready_col].trim();
            let ready = line[ready_col..].trim() == "YES";

            let product = match component {
                "Houdini" => Product::Houdini(HoudiniInstallation::new(path, version, ready)?),
                "hserver" => Product::HServer(Installation::new(path, version, ready)?),
                "License Server" => {
                    Product::LicenseServer(Installation::new(path, version, ready)?)
                }
                "HQueue Server" => Product::HQueueServer(Installation::new(path, version, ready)?),
                "HQueue Client" => Product::HQueueClient(Installation::new(path, version, ready)?),
                other => bail!("Unknown component: {other}"),
            };

            products.push(product);
        }

        Ok(products)
    }

    #[cfg(target_os = "macos")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/Applications/Houdini Launcher.app/Contents/MacOS/houdini_installer"),
            data_path.join("installer/Houdini Launcher.app/Contents/MacOS/houdini_installer"),
        ]
    }

    #[cfg(target_os = "linux")]
    fn candidate_paths(data_path: &Path) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/opt/sidefx/launcher/bin/houdini_installer"),
            data_path.join("installer/houdini_installer"),
        ]
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn candidate_paths(_data_path: &Path) -> Vec<PathBuf> {
        vec![]
    }
}
