use anyhow::{Result, bail};
use clap::Parser;
use commands::{Cli, Commands};
use console::style;
use project::Project;
use std::env;
use std::path::{Path, PathBuf};

mod commands;
pub mod config;
mod credentials;
pub mod elevated_command;
mod hou;
mod installations;
mod launcher;
pub mod package;
mod project;
mod sidefx;
mod houdini;
mod utils;

pub fn main() -> Result<()> {
    env_logger::init();
    log::info!("Initializing...");

    let cli = Cli::parse();
    let hou = hou::Context::new()?;
    let cwd = env::current_dir()?;

    let needs_project = matches!(
        cli.command,
        Some(Commands::Run(_) | Commands::Package(_)) | None
    );

    let default_launch = match &cli.command {
        None => Some(parse_default_launch(
            &cwd,
            cli.file.as_deref(),
            &cli.houdini_args,
        )),
        _ => None,
    };

    let discovery_start = default_launch
        .as_ref()
        .map(|d| d.discovery_start.clone())
        .unwrap_or_else(|| cwd.clone());

    let project = if needs_project {
        Project::discover(&discovery_start)?
    } else {
        None
    };

    if let Some(d) = &default_launch {
        if d.require_project && project.is_none() {
            bail!(
                "{} is not inside a Houdini project (no hproject.json found)",
                d.discovery_start.display()
            );
        }
    }

    // --version from the subcommand, or top-level for the default launch
    let user_version = match &cli.command {
        None => cli.version.clone(),
        Some(Commands::Run(cmd)) => cmd.version.clone(),
        Some(Commands::Package(cmd)) => cmd.version.clone(),
        Some(Commands::Init(cmd)) => cmd.version.clone(),
        Some(_) => None,
    };

    // explicit --global package scope escapes the project version pin
    let global_package_scope = matches!(&cli.command, Some(Commands::Package(cmd)) if cmd.global);

    let version_filter = match (&project, user_version) {
        (Some(_), Some(v)) if global_package_scope => Some(v),
        (Some(p), user_filter) => {
            if let Some(v) = &user_filter {
                let project_version = p.houdini_version().unwrap_or("(unset)");
                eprintln!(
                    "{} --version={} is ignored inside a project; using project's houdini_version={}",
                    style("warning:").yellow().bold(),
                    style(v).yellow(),
                    style(project_version).yellow(),
                );
            }
            p.houdini_version().map(|s| s.to_string())
        }
        (None, user_filter) => user_filter,
    };

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            cmd.run(houdini, project.as_ref())?;
        }
        Some(Commands::Sidefx(cmd)) => {
            cmd.run(&hou)?;
        }
        Some(Commands::Launcher(cmd)) => cmd.run(&hou)?,
        Some(Commands::Init(cmd)) => cmd.run(&hou, version_filter.as_deref())?,
        Some(Commands::Package(cmd)) => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            cmd.run(&hou, houdini, project.as_ref())?;
        }
        Some(Commands::Install(cmd)) => cmd.run(&hou)?,
        Some(Commands::Uninstall(cmd)) => cmd.run(&hou)?,
        Some(Commands::List(cmd)) => cmd.run(&hou)?,
        Some(Commands::Login(cmd)) => cmd.run(&hou)?,
        Some(Commands::Logout(cmd)) => cmd.run(&hou)?,
        Some(Commands::Eula(cmd)) => cmd.run(&hou)?,
        Some(Commands::Config(cmd)) => cmd.run(&hou)?,
        None => {
            let houdini = hou.resolve_houdini(version_filter.as_deref())?;
            let forward_args = default_launch.map(|d| d.forward_args).unwrap_or_default();
            houdini.launch(&forward_args, project.as_ref(), cli.attach)?;
        }
    }

    Ok(())
}

struct DefaultLaunch {
    discovery_start: PathBuf,
    forward_args: Vec<String>,
    require_project: bool,
}

fn parse_default_launch(cwd: &Path, file: Option<&str>, houdini_args: &[String]) -> DefaultLaunch {
    let Some(first) = file else {
        return DefaultLaunch {
            discovery_start: cwd.to_path_buf(),
            forward_args: houdini_args.to_vec(),
            require_project: false,
        };
    };

    let p = Path::new(first);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };

    if abs.is_dir() {
        DefaultLaunch {
            discovery_start: abs,
            forward_args: houdini_args.to_vec(),
            require_project: true,
        }
    } else {
        let start = abs
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.to_path_buf());
        let mut forward = Vec::with_capacity(1 + houdini_args.len());
        forward.push(first.to_string());
        forward.extend(houdini_args.iter().cloned());
        DefaultLaunch {
            discovery_start: start,
            forward_args: forward,
            require_project: false,
        }
    }
}
