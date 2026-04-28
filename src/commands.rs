pub use clap::{Parser, Subcommand};

pub mod init;
mod package;
mod run;
mod sidefx;
pub mod setup;

#[derive(Subcommand)]
pub enum Commands {
    Setup,
    Init(init::InitCmd),
    Run(run::Run),
    Sidefx(sidefx::SideFX),
    Package(package::PackageCmd),
}

#[derive(Parser)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    #[arg(short, long)]
    pub version: Option<String>,
    /// Arguments forwarded to Houdini when no subcommand is given (e.g. a .hip file).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub houdini_args: Vec<String>,
}
