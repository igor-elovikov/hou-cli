use crate::settings::Settings;
use crate::sidefx::{HoudiniLauncher, Platform, Product};
use anyhow::{Context, Result};
use console::style;

/// Updates the SideFX launcher to the latest production build.
pub fn update(ctx: &crate::hou::Context) -> Result<()> {
    let current = ctx.installer()?.version()?;

    let settings = Settings::load(&ctx.config_dir)?;
    let (client_id, client_secret) = settings.require_oauth()?;
    let client = crate::sidefx::Client::new(&client_id, &client_secret)?;

    let launcher_platform = Platform::host()?;
    // let launcher_platform = match host_platform {
    //     Platform::Macos | Platform::MacosxArm64 => Platform::Macos,
    //     other => other,
    // };

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

    // Refresh the launcher where it was discovered, falling back to the default
    // install location. A launcher outside the data dir is a system install
    // (e.g. /opt/sidefx/launcher); reinstalling there needs elevation, which
    // install_launcher handles.
    let target = ctx
        .installer()?
        .launcher_dir()
        .unwrap_or_else(|| crate::installer::default_launcher_dir(&ctx.data_dir));
    let kind = if target.starts_with(&ctx.data_dir) {
        "launcher"
    } else {
        "system launcher"
    };

    println!(
        "Updating {} {} -> {} at {}",
        kind,
        style(&current).yellow(),
        style(&latest.version).green(),
        style(target.display()).dim(),
    );

    let latest_major = format!("{}.{}", latest.version.major, latest.version.minor);
    client.install_launcher(
        HoudiniLauncher::Default,
        latest_major,
        &ctx.data_dir,
        &target,
    )?;

    Ok(())
}
