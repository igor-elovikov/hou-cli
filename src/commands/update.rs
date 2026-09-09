use crate::launcher::LauncherProduct;
use clap::Args;

#[derive(Args)]
struct UpdateCmd {
    /// Product to update
    #[arg(short, long, value_enum, default_value_t = LauncherProduct::Houdini)]
    product: LauncherProduct,

    #[arg(short, long)]
    version: Option<String>,

    #[arg(short, long)]
    to: Option<String>,
}

impl UpdateCmd {
    pub fn run(self, ctx: &crate::hou::Context) -> anyhow::Result<()> {
        let product = ctx.resolve_product(self.product, self.version.as_deref())?;

        Ok(())
    }
}
