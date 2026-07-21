use crate::hou::Context;
use crate::sidefx::Houdini::Default;
use crate::sidefx::{Product, Release};
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

        let unique_builds = builds
            .iter()
            .filter(|b| req.matches(&b.version))
            .unique_by(|b| b.version.clone())
            .collect::<Vec<_>>();

        if unique_builds.is_empty() {
            println!("No version found");
        } else {
            for b in &unique_builds {
                let kind = match b.release {
                    Release::Gold => {"Production"}
                    Release::Development => {"Daily"}
                };
                println!("{} {}", b.version, kind);
            }
        }

        Ok(())
    }
}
