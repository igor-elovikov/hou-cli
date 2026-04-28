use anyhow::Result;
use clap::Parser;
use commands::{Cli, Commands, setup::setup};
use project::Project;
use std::env;
use std::path::{Path, PathBuf};

mod commands;
mod hou;
mod installations;
mod installer;
pub mod package;
mod project;
mod sidefx;

pub fn main() -> Result<()> {
    env_logger::init();
    log::info!("Initializing...");

    let cli = Cli::parse();
    let hou = hou::Context::new()?;

    let needs_project = matches!(
        cli.command,
        Some(Commands::Run(_) | Commands::Package(_)) | None
    );
    let discovery_start = project_discovery_start(&cli)?;
    let project = if needs_project {
        Project::discover(&discovery_start)?
    } else {
        None
    };

    let version_filter = match (&project, cli.version.as_deref()) {
        (Some(p), user_filter) => {
            if user_filter.is_some() {
                log::warn!("--version is ignored inside a project; using project's houdini_version");
            }
            p.houdini_version().map(|s| s.to_string())
        }
        (None, user_filter) => user_filter.map(|s| s.to_string()),
    };

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            cmd.run(houdini, project.as_ref())?;
        }
        Some(Commands::Sidefx(cmd)) => {
            cmd.run()?;
        }
        Some(Commands::Setup) => setup(&hou)?,
        Some(Commands::Init(cmd)) => cmd.run(&hou)?,
        Some(Commands::Package(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            cmd.run(&hou, houdini, project.as_ref())?;
        }
        None => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            houdini.launch_houdini(&cli.houdini_args, project.as_ref())?;
        }
    }

    Ok(())
}

fn project_discovery_start(cli: &Cli) -> Result<PathBuf> {
    let cwd = env::current_dir()?;
    let file_arg = cli
        .houdini_args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(Path::new);
    Ok(match file_arg {
        Some(p) => {
            let abs = if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) };
            abs.parent().map(Path::to_path_buf).unwrap_or(abs)
        }
        None => cwd,
    })
}
