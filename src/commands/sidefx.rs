use crate::hou::Context;
use crate::settings::Settings;
use crate::sidefx::{BuildSpec, Platform, Product};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

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

#[derive(Args)]
struct Download {
    product: Product,
    /// major.minor (e.g. 21.0) for the latest production build, or
    /// major.minor.build (e.g. 21.0.729) for a specific build.
    version: String,
    /// Target platform (defaults to the host platform).
    #[arg(short, long)]
    platform: Option<Platform>,
    /// Directory to download into (defaults to the current directory).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum SideFXCommand {
    Builds(Builds),
    Download(Download),
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
            SideFXCommand::Download(args) => {
                let (version, build) = split_version(&args.version);
                let platform = match args.platform {
                    Some(p) => p,
                    None => Platform::host()?,
                };

                let info = client
                    .build_download(args.product, version, build)
                    .platform(platform)
                    .send()?;

                let dir = match args.output {
                    Some(d) => d,
                    None => std::env::current_dir()?,
                };

                println!("Downloading {} ({} bytes)...", info.filename, info.size);
                let path = client.download_build(&info, &dir)?;
                println!("Saved to {}", path.display());

                Ok(())
            }
        }
    }
}

/// Splits `major.minor.build` into the API version and an explicit build,
/// falling back to `major.minor` plus the latest production build.
fn split_version(s: &str) -> (String, BuildSpec) {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() >= 3 {
        if let Ok(build) = parts[2].parse::<u32>() {
            return (format!("{}.{}", parts[0], parts[1]), BuildSpec::Number(build));
        }
    }
    (s.to_string(), BuildSpec::Production)
}
