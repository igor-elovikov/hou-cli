use crate::hou::Context;
use crate::launcher::LauncherProduct;
use crate::sidefx::{Houdini, Platform, Product, Status};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use semver::Version;

#[derive(Args)]
pub struct InstallCmd {
    /// Full or partial version (e.g. 21.0.729 or 21.0); latest when omitted.
    version: Option<String>,
    /// Product to install
    #[arg(short, long, value_enum, default_value_t = LauncherProduct::Houdini)]
    product: LauncherProduct,
    #[arg(long)]
    build_option: Option<String>,
    #[arg(long)]
    avahi: bool,
    /// Install the latest daily build instead of production.
    #[arg(short, long)]
    daily: bool,
}

impl InstallCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let version = self.resolve_version(ctx)?;

        let already_installed = ctx.products.iter().any(|p| {
            p.launcher_product().is_ok_and(|lp| lp == self.product) && p.version() == &version
        });

        if already_installed {
            println!("{} {} is already installed", style(&self.product).cyan(), style(&version).cyan());
            return Ok(());
        }

        println!("Installing {} {}...", style(&self.product).cyan(), style(&version).green());
        ctx.launcher()?.install_product(
            ctx,
            &version.to_string(),
            &self.product,
        )?;
        println!("Installed {} {}", style(&self.product).cyan(), style(&version).green());
        Ok(())
    }

    /// Resolves the version to install; queries the SideFX API unless a full version is given.
    fn resolve_version(&self, ctx: &Context) -> Result<Version> {
        if let Some(v) = &self.version {
            return Ok(Version::parse(v).map_err(|_| {
                anyhow::anyhow!(
                    "Invalid version '{v}'. To install a product you need to specify full version"
                )
            })?);
        }

        let client = ctx.sidefx_client()?;

        let mut builds = client
            .builds(Product::Houdini(Houdini::Default))
            .platform(Platform::host()?);
        if let Some(v) = &self.version {
            builds = builds.version(v.clone());
        }
        if !self.daily {
            builds = builds.only_production();
        }

        let kind = if self.daily { "daily" } else { "production" };
        builds
            .send()?
            .into_iter()
            .filter(|b| matches!(b.status, Status::Good))
            .max_by_key(|b| b.version.clone())
            .map(|b| b.version)
            .ok_or_else(|| match &self.version {
                Some(v) => anyhow!("no {kind} Houdini builds found matching '{v}'"),
                None => anyhow!("no {kind} Houdini builds found"),
            })
    }
}


