use clap::{Parser, Subcommand};
use crate::hou::Context as HouContext;
use crate::credentials::CredentialSettings;
use crate::sidefx::{HoudiniLauncher, Platform, Product};
use anyhow::{Context, Result};
use console::style;
use crate::launcher::Launcher;

#[derive(Subcommand)]
pub enum LauncherAction {
    /// Set up SideFX Launcher for the first time (installs the launcher).
    Setup,
    /// Update SideFX Launcher
    Update,
    /// Run houdini_installer (SideFX Launcher CLI official tool)
    Cli
}

#[derive(Parser)]
#[command(
    about = "Manage the SideFX Launcher",
    long_about = "Manage the SideFX Launcher.\n\nWith no subcommand, runs the launcher GUI directly."
)]
pub struct LauncherCmd {
    #[command(subcommand)]
    pub action: Option<LauncherAction>,

    /// Arguments forwarded to CLI and launcher
    #[arg(last = true, global = true)]
    pub args: Vec<String>,
}


/// Set up the SideFX Launcher
fn setup(ctx: &HouContext) -> Result<()> {
    let creds = CredentialSettings::load(&ctx.config_dir)?;

    if ctx.installer.is_some() {
        println!("SideFX Launcher is already installed, skipping setup.");
        return Ok(());
    }

    let (client_id, client_secret) = creds.require_oauth()?;
    let client = crate::sidefx::Client::new(&client_id, &client_secret)?;
    let launcher = Product::HoudiniLauncher(HoudiniLauncher::Default);

    let host_platform = Platform::host()?;

    let builds = client
        .builds(launcher)
        .platform(host_platform)
        .only_production()
        .send()?;

    let latest_build = builds
        .iter()
        .max_by_key(|b| &b.version)
        .context("No build found for launcher")?;

    let latest_version = &latest_build.version;
    let latest_major = format!("{}.{}", latest_version.major, latest_version.minor);

    println!("Found launcher version: {}", style(latest_version).green());

    client.install_launcher(
        HoudiniLauncher::Default,
        latest_major,
        &ctx.data_dir,
        &Launcher::install_path(),
    )?;

    Ok(())
}

/// Updates the SideFX launcher to the latest production build.
pub fn update(ctx: &crate::hou::Context) -> Result<()> {
    let current = ctx.installer()?.version()?;

    let settings = CredentialSettings::load(&ctx.config_dir)?;
    let (client_id, client_secret) = settings.require_oauth()?;
    let client = crate::sidefx::Client::new(&client_id, &client_secret)?;

    let launcher_platform = Platform::host()?;

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

    // Refresh the launcher where it was discovered
    let target = ctx
        .installer()?
        .current_install_path()
        .context("Failed to get launcher dir")?;

    println!(
        "Updating Launcher {} -> {} at {}",
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

impl LauncherCmd {
    pub fn run(&self, ctx: &HouContext) -> Result<()> {
        match &self.action {
            Some(LauncherAction::Setup) => setup(ctx)?,
            Some(LauncherAction::Update) => update(ctx)?,
            Some(LauncherAction::Cli) => {
                let installer = ctx.installer()?;
                installer.run_installer_bare(&self.args)?;
            },
            None => {
                let installer = ctx.installer()?;
                installer.run_launcher(&self.args)?;
            },
        }
        Ok(())
    }
}