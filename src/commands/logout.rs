use crate::hou::Context;
use crate::settings::Settings;
use anyhow::Result;
use clap::Args;
use console::style;

#[derive(Args)]
pub struct LogoutCmd;

const ENV_KEYS: &[&str] = &[
    "HOU_USERNAME",
    "HOU_PASSWORD",
    "HOU_CLIENT_ID",
    "HOU_CLIENT_SECRET",
];

impl LogoutCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let removed = Settings::delete(&ctx.config_dir)?;
        if removed {
            println!("Cleared SideFX settings");
        } else {
            println!("No SideFX settings to clear");
        }

        let active_env: Vec<&&str> = ENV_KEYS
            .iter()
            .filter(|k| std::env::var(k).is_ok())
            .collect();
        if !active_env.is_empty() {
            let names: Vec<String> = active_env.iter().map(|k| k.to_string()).collect();
            eprintln!(
                "{} env vars still set: {}",
                style("warning:").yellow().bold(),
                names.join(", "),
            );
        }
        Ok(())
    }
}
