pub use clap::{Parser, Subcommand};

pub mod init;
pub mod list;
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
    /// List installed Houdini products.
    List(list::ListCmd),
}

#[derive(Parser)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    #[arg(short, long)]
    pub version: Option<String>,
    /// Keep stdio attached to the terminal and wait for Houdini to exit.
    #[arg(short, long)]
    pub attach: bool,
    /// Optional file (e.g. a .hip file) or project directory to open.
    pub file: Option<String>,
    /// Arguments forwarded to Houdini; everything after `--`.
    #[arg(last = true)]
    pub houdini_args: Vec<String>,
}
