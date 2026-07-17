use crate::credentials::CredentialSettings;
use crate::hou::Context;
use anyhow::Result;
use clap::{Args, Subcommand};
use console::style;

#[derive(Args)]
pub struct EulaCmd {
    #[command(subcommand)]
    command: EulaAction,
}

#[derive(Subcommand)]
enum EulaAction {
    /// Add an accepted EULA date (e.g. SideFX-2024-11-13).
    Add { date: String },
    /// Remove all accepted EULA dates.
    Clear,
    /// List accepted EULA dates.
    List,
}

impl EulaCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let mut settings = CredentialSettings::load(&ctx.config_dir)?;
        match self.command {
            EulaAction::Add { date } => {
                if settings.add_eula(&date) {
                    settings.save()?;
                    println!("Added EULA {}", style(&date).cyan());
                } else {
                    println!("EULA {} already accepted", style(&date).dim());
                }
            }
            EulaAction::Clear => {
                settings.clear_eulas();
                settings.save()?;
                println!("Cleared accepted EULAs");
            }
            EulaAction::List => {
                let eulas = settings.eulas();
                if eulas.is_empty() {
                    println!("{}", style("(no accepted EULAs)").dim());
                } else {
                    println!("{}", style("Accepted EULAs").bold());
                    for d in eulas {
                        println!("  {} {}", style("•").dim(), d);
                    }
                }
            }
        }
        Ok(())
    }
}
