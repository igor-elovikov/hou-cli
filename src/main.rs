use anyhow::Result;
use clap::Parser;
use commands::{Cli, Commands, setup::setup};

mod commands;
mod hou;
mod installations;
mod installer;
pub mod package;
mod sidefx;

pub fn main() -> Result<()> {
    env_logger::init();
    log::info!("Initializing...");

    let cli = Cli::parse();
    let hou = hou::Context::new()?;
    let version_filter = cli.version.as_deref();

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter)?;
            cmd.run(houdini)?;
        }
        Some(Commands::Sidefx(cmd)) => {
            cmd.run()?;
        }
        Some(Commands::Setup) => setup(&hou)?,
        Some(Commands::Package(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter)?;
            cmd.run(&hou, houdini)?;
        }
        None => {
            let houdini = hou.resolve_houdini(version_filter)?;
            houdini.launch_houdini(&cli.houdini_args)?;
        }
    }

    Ok(())
}
