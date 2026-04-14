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
    pub fn run(self) -> Result<()> {
        let client = crate::sidefx::Client::new()?;

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
