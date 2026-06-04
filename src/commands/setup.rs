use crate::settings::Settings;
use crate::sidefx::{HoudiniLauncher, Platform, Product};
use anyhow::{Context, Result};
use console::style;

pub fn setup(ctx: &crate::hou::Context) -> Result<()> {
    let settings = Settings::load(&ctx.config_dir)?;
    let (client_id, client_secret) = settings.require_oauth()?;
    let client = crate::sidefx::Client::new(&client_id, &client_secret)?;
    let launcher = Product::HoudiniLauncher(HoudiniLauncher::Default);

    let host_platform = Platform::host()?;

    let launcher_platform = match host_platform {
        Platform::Macos | Platform::MacosxArm64 => Platform::Macos,
        other => other,
    };

    let builds = client
        .builds(launcher)
        .platform(launcher_platform)
        .only_production()
        .send()?;

    let latest_build = builds
        .iter()
        .max_by_key(|b| &b.version)
        .context("No build found for launcher")?;

    let latest_version = &latest_build.version;
    let latest_major = format!("{}.{}", latest_version.major, latest_version.minor);

    println!("Found launcher version: {}", style(latest_version).green());

    let target = crate::installer::default_launcher_dir(&ctx.data_dir);

    client.install_launcher(
        HoudiniLauncher::Default,
        latest_major,
        &ctx.data_dir,
        &target,
    )?;

    Ok(())
}
