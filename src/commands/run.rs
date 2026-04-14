use crate::products::HoudiniInstallation;
use anyhow::{Context, Result};
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct Run {
    /// This captures everything after 'run'
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

impl Run {
    pub fn run(&self, houdini: &HoudiniInstallation) -> Result<()> {
        let (command, args) = self.args.split_first().context("No command provided")?;
        let mut command = Command::new(command);
        command.args(args);
        houdini.run(command)?;

        Ok(())
    }
}
