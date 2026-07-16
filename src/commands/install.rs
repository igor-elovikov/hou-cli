use crate::hou::Context;
use crate::installations::InstalledProduct;
use crate::settings::Settings;
use crate::sidefx::{Houdini, Platform, Product, Status};
use anyhow::{anyhow, bail, Context as _, Result};
use clap::Args;
use console::style;
use semver::Version;
use std::io::Write;
use tempfile::NamedTempFile;

#[derive(Args)]
pub struct InstallCmd {
    /// Full or partial version (e.g. 21.0.729 or 21.0); latest when omitted.
    version: Option<String>,
    /// Install the latest daily build instead of production.
    #[arg(short, long)]
    daily: bool,
}

impl InstallCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let settings = Settings::load(&ctx.config_dir)?;
        let version = self.resolve_version(&settings)?;

        let already_installed = ctx.products.iter().any(|p| match p {
            InstalledProduct::Houdini(h) => h.version == version,
            _ => false,
        });
        if already_installed {
            println!("Houdini {} is already installed", style(&version).cyan());
            return Ok(());
        }

        let eulas = settings.eulas();
        if eulas.is_empty() {
            bail!(
                "no accepted SideFX EULA dates; view the license and run `hou eula add SideFX-YYYY-MM-DD`"
            );
        }

        let settings_file = credentials_ini(&settings)?;
        println!("Installing Houdini {}...", style(&version).green());
        ctx.installer()?
            .install_houdini(&version.to_string(), settings_file.path(), &eulas)?;
        println!("Installed Houdini {}", style(&version).green());
        Ok(())
    }

    /// Resolves the version to install; queries the SideFX API unless a full version is given.
    fn resolve_version(&self, settings: &Settings) -> Result<Version> {
        if let Some(v) = &self.version {
            if let Ok(full) = Version::parse(v) {
                return Ok(full);
            }
        }

        let (client_id, client_secret) = settings.require_oauth()?;
        let client = crate::sidefx::Client::new(&client_id, &client_secret)?;

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

/// Writes credentials and accepted EULAs to a temp ini for houdini_installer.
fn credentials_ini(settings: &Settings) -> Result<NamedTempFile> {
    let mut text = String::new();
    if let (Some(id), Some(secret)) = (settings.client_id(), settings.client_secret()) {
        text.push_str(&format!("client_id={id}\nclient_secret={secret}\n"));
    } else if let (Some(user), Some(pass)) = (settings.username(), settings.password()) {
        text.push_str(&format!("username={user}\npassword={pass}\n"));
    } else {
        bail!(
            "no SideFX credentials; run `hou login oauth <client_id> <client_secret>` or `hou login user <username> <password>`"
        );
    }

    let eulas = settings.eulas();
    if !eulas.is_empty() {
        text.push_str(&format!("accept_eula={}\n", eulas.join(" ")));
    }

    let mut file = tempfile::Builder::new()
        .prefix("hou-install-")
        .suffix(".ini")
        .tempfile()
        .context("failed to create temp settings file")?;
    file.write_all(text.as_bytes())
        .context("failed to write temp settings file")?;
    Ok(file)
}
