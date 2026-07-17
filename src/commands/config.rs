use crate::config::{Config, KEYS};
use crate::hou::Context;
use anyhow::Result;
use clap::{Args, Subcommand};
use console::style;

#[derive(Args)]
pub struct ConfigCmd {
    #[command(subcommand)]
    command: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print a config value.
    Get { key: String },
    /// Set a config value.
    Set { key: String, value: String },
    /// Remove a config value.
    Unset { key: String },
    /// List all config values.
    List,
    /// Print the config file path.
    Path,
}

impl ConfigCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let mut config = Config::load(&ctx.config_dir)?;
        match self.command {
            ConfigAction::Get { key } => match config.get(&key)? {
                Some(value) => println!("{value}"),
                None => println!("{}", style("(unset)").dim()),
            },
            ConfigAction::Set { key, value } => {
                config.set(&key, &value)?;
                config.save(&ctx.config_dir)?;
                let stored = config.get(&key)?.unwrap_or_default();
                println!("{} = {}", style(&key).cyan(), stored);
            }
            ConfigAction::Unset { key } => {
                config.unset(&key)?;
                config.save(&ctx.config_dir)?;
                println!("Unset {}", style(&key).cyan());
            }
            ConfigAction::List => {
                println!("{}", style("Config").bold());
                for key in KEYS {
                    match config.get(key)? {
                        Some(value) => println!("  {} = {}", style(key).cyan(), value),
                        None => println!("  {} = {}", style(key).cyan(), style("(unset)").dim()),
                    }
                }
            }
            ConfigAction::Path => println!("{}", Config::path(&ctx.config_dir).display()),
        }
        Ok(())
    }
}
