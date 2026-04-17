pub use clap::{Parser, Subcommand};

pub mod init;
mod run;
mod sidefx;

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Download,
    Install,
    Run(run::Run),
    Sidefx(sidefx::SideFX),
}

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    #[arg(short, long)]
    pub version: Option<String>,
}
