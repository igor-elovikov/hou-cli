pub use clap::{Parser, Subcommand};

pub mod init;
mod package;
mod run;
mod sidefx;

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Run(run::Run),
    Sidefx(sidefx::SideFX),
    Package(package::PackageCmd),
}

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    #[arg(short, long)]
    pub version: Option<String>,
}
