pub use clap::{Parser, Subcommand};

mod run;
pub mod sidefx;

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
}
