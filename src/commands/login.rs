use crate::credentials::CredentialSettings;
use crate::hou::Context;
use anyhow::Result;
use clap::{Args, Subcommand};
use console::style;

#[derive(Args)]
pub struct LoginCmd {
    #[command(subcommand)]
    command: LoginKind,
}

#[derive(Subcommand)]
enum LoginKind {
    /// Store SideFX account username/password (used by houdini_installer).
    User(UserArgs),
    /// Store SideFX Web API OAuth client credentials.
    Oauth(OauthArgs),
}

#[derive(Args)]
struct UserArgs {
    username: String,
    password: String,
}

#[derive(Args)]
struct OauthArgs {
    client_id: String,
    client_secret: String,
}

impl LoginCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let mut settings = CredentialSettings::load(&ctx.config_dir)?;
        match self.command {
            LoginKind::User(a) => {
                settings.set_user_login(&a.username, &a.password);
                settings.save()?;
                println!(
                    "Stored username/password for {} in {}",
                    style(&a.username).cyan(),
                    style(settings.path().display()).dim(),
                );
            }
            LoginKind::Oauth(a) => {
                settings.set_oauth(&a.client_id, &a.client_secret);
                settings.save()?;
                println!(
                    "Stored OAuth credentials in {}",
                    style(settings.path().display()).dim(),
                );
            }
        }
        Ok(())
    }
}
