use crate::installations::HoudiniInstallation;
use crate::pip::Pip;
use crate::project::Project;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct PipCmd {
    #[command(subcommand)]
    pub action: PipAction,
}

#[derive(Subcommand)]
pub enum PipAction {
    /// Install pip requirements (e.g. `requests`, `numpy<2`) into the project.
    #[command(visible_alias = "i")]
    Install {
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Remove pip requirements from the project.
    #[command(visible_alias = "rm")]
    Uninstall {
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Update the given packages, or all when none are given, to the newest allowed versions.
    #[command(visible_alias = "u")]
    Update { packages: Vec<String> },
}

impl PipCmd {
    pub fn run(self, houdini: &HoudiniInstallation, project: &Project) -> Result<()> {
        let mut pip = Pip::open(houdini, project)?;
        match self.action {
            PipAction::Install { packages } => pip.install(&packages),
            PipAction::Uninstall { packages } => pip.uninstall(&packages),
            PipAction::Update { packages } => pip.update(&packages),
        }
    }
}
