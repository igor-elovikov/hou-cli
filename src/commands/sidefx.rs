use crate::hou::Context;
use crate::settings::Settings;
use crate::sidefx::{Platform, Product};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
struct Builds {
    product: Product,
    #[arg(short, long)]
    version: Option<String>,
    #[arg(short, long)]
    all: bool,
    #[arg(short, long)]
    platform: Option<Platform>,
}

#[derive(Subcommand)]
enum SideFXCommand {
    Builds(Builds),
    Download,
}

#[derive(Args)]
pub struct SideFX {
    #[command(subcommand)]
    command: SideFXCommand,
}

impl SideFX {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let settings = Settings::load(&ctx.config_dir)?;
        let (client_id, client_secret) = settings.require_oauth()?;
        let client = crate::sidefx::Client::new(&client_id, &client_secret)?;

        match self.command {
            SideFXCommand::Builds(args) => {
                let mut builds = client.builds(args.product);

                if let Some(version) = args.version {
                    builds = builds.version(version);
                }

                if let Some(platform) = args.platform {
                    builds = builds.platform(platform);
                }

                if !args.all {
                    builds = builds.only_production();
                }

                let result = builds.send()?;

                println!("{}", serde_json::to_string_pretty(&result)?);

                Ok(())
            }
            SideFXCommand::Download => Ok(()),
        }
    }
}
