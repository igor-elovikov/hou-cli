use crate::launcher::LauncherProduct;
use clap::Args;

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

        let client = ctx.sidefx_client()?;
        let latest_version = client.latest_version_for(version.major, version.minor, true)?;

        println!("Found latest version {} for {} {}", latest_version, self.product, version);

        let launcher = ctx.launcher()?;
        launcher.modify(
            ctx,
            product,
            &format!("{}.{}.{}", latest_version.major, latest_version.minor, latest_version.patch),
        )?;

        println!("Updated {} to version {}", self.product, latest_version);

        Ok(())
    }
}
