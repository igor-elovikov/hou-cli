pub use clap::{Parser, Subcommand};

mod run;

#[derive(Subcommand)]
pub enum Commands {
    Run(run::Run),
}

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
}
