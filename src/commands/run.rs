use crate::installations::HoudiniInstallation;
use crate::project::Project;
use anyhow::{Context, Result};
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct Run {
    /// Houdini version filter (ignored inside a project).
    #[arg(short, long, global = true)]
    pub version: Option<String>,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

impl Run {
    pub fn run(self, houdini: &HoudiniInstallation, project: Option<&Project>) -> Result<()> {
        let (command, args) = self.args.split_first().context("No command provided")?;
        let mut command = Command::new(command);
        command.args(args);
        houdini.run(command, project)?;

        Ok(())
    }
}
