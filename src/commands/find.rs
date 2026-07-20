use crate::hou::Context;
use crate::sidefx::Houdini::Default;
use crate::sidefx::Product;
use crate::utils::normalize_version_filter;
use anyhow::Result;
use clap::Args;
use itertools::Itertools;

#[derive(Args)]
pub struct FindCmd {
    version: Option<String>,
}

impl FindCmd {
    pub fn run(&self, ctx: &Context) -> Result<()> {
        let req = normalize_version_filter(self.version.as_deref())?;

        let client = ctx.sidefx_client()?;
        let builds = client.builds(Product::Houdini(Default)).send()?;

        let versions = builds
            .iter()
            .filter(|b| req.matches(&b.version))
            .map(|b| b.version.clone())
            .unique()
            .collect::<Vec<_>>();

        if versions.is_empty() {
            println!("No version found");
        } else {
            for version in &versions {
                println!("{}", version);
            }
        }

        Ok(())
    }
}
