use crate::hou::Context;
use anyhow::Result;
use clap::Args;
use console::style;

#[derive(Args)]
pub struct UninstallCmd {
    /// Full or partial version of an installed Houdini (e.g. 21.0.729 or 21.0).
    version: String,
}

impl UninstallCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let houdini = ctx.resolve_houdini(Some(&self.version))?;
        println!(
            "Uninstalling Houdini {} at {}...",
            style(&houdini.version).green(),
            style(houdini.path.display()).dim(),
        );
        ctx.installer()?.uninstall(&houdini.path)?;
        println!("Uninstalled Houdini {}", style(&houdini.version).green());
        Ok(())
    }
}
