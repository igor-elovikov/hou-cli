use crate::settings::Settings;
use crate::sidefx::{HoudiniLauncher, Platform, Product};
use anyhow::{Context, Result};
use console::style;

/// Updates the SideFX launcher to the latest production build.
pub fn update(ctx: &crate::hou::Context) -> Result<()> {
    let current = ctx.installer.version()?;

    let settings = Settings::load(&ctx.config_dir)?;
    let (client_id, client_secret) = settings.require_oauth()?;
    let client = crate::sidefx::Client::new(&client_id, &client_secret)?;

    let host_platform = Platform::host()?;
    let launcher_platform = match host_platform {
        Platform::Macos | Platform::MacosxArm64 => Platform::Macos,
        other => other,
    };

    let builds = client
        .builds(Product::HoudiniLauncher(HoudiniLauncher::Default))
        .platform(launcher_platform)
        .only_production()
        .send()?;

    let latest = builds
        .iter()
        .max_by_key(|b| &b.version)
        .context("No launcher builds found")?;

    if latest.version <= current {
        println!("Launcher {} is up to date", style(&current).green());
        return Ok(());
    }

    println!(
        "Updating launcher {} -> {}",
        style(&current).yellow(),
        style(&latest.version).green(),
    );

    let latest_major = format!("{}.{}", latest.version.major, latest.version.minor);
    client.install_launcher(HoudiniLauncher::Default, latest_major, &ctx.data_dir)?;

    Ok(())
}
