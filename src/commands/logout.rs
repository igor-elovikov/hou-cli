use crate::hou::Context;
use crate::settings::Settings;
use anyhow::Result;
use clap::Args;
use console::style;

#[derive(Args)]
pub struct LogoutCmd;

impl LogoutCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let removed = Settings::delete(&ctx.config_dir)?;
        if removed {
            println!("Cleared SideFX WebAPI settings");
        } else {
            println!("No SideFX WebAPI settings to clear");
        }

        Ok(())
    }
}
