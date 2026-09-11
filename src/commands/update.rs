use crate::launcher::LauncherProduct;
use clap::Args;
use console::style;

#[derive(Args)]
pub struct UpdateCmd {
    /// Product to update
    #[arg(short, long, value_enum, default_value_t = LauncherProduct::Houdini)]
    product: LauncherProduct,

    #[arg(short, long)]
    version: Option<String>,

    #[arg(short, long)]
    to: Option<String>,
}

impl UpdateCmd {
    pub fn run(&self, ctx: &crate::hou::Context) -> anyhow::Result<()> {
        let product = ctx.resolve_product(self.product, self.version.as_deref())?;
        let version = product.version();

        let target_version = match &self.to {
            Some(to) => semver::Version::parse(to)?,
            None => {
                let client = ctx.sidefx_client()?;
                let latest = client.latest_version_for(version.major, version.minor, true)?;
                println!(
                    "Found latest version {} for {} {}",
                    latest, self.product, version
                );
                latest
            }
        };

        let launcher = ctx.launcher()?;
        launcher.modify(
            ctx,
            product,
            &format!(
                "{}.{}.{}",
                target_version.major, target_version.minor, target_version.patch
            ),
        )?;

        println!(
            "Updated {}:  {} -> {}",
            style(self.product).bold(),
            style(version).yellow(),
            style(target_version).green()
        );

        Ok(())
    }
}
