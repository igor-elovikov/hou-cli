use crate::credentials::CredentialSettings;
use crate::hou::Context;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct LogoutCmd;

impl LogoutCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let removed = CredentialSettings::delete(&ctx.config_dir)?;
        if removed {
            println!("Cleared SideFX WebAPI settings");
        } else {
            println!("No SideFX WebAPI settings to clear");
        }

        Ok(())
    }
}
