use anyhow::Result;
use clap::Parser;
use commands::{Cli, Commands, init::init};

mod commands;
mod hou;
mod installations;
mod installer;
pub mod package;
mod sidefx;

pub fn main() -> Result<()> {
    env_logger::init();
    log::info!("Initializing...");

    let hou = hou::Context::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.latest_houdini()?;
            cmd.run(houdini)?;
        }
        Some(Commands::Sidefx(cmd)) => {
            cmd.run()?;
        }
        Some(Commands::Init) => init(&hou)?,
        Some(Commands::Package(cmd)) => cmd.run(&hou)?,
        None => {
            let houdini = hou.latest_houdini()?;
            houdini.launch_houdini()?;
        }
    }

    Ok(())
}
