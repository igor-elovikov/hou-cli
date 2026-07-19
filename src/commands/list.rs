use std::path::PathBuf;
use crate::hou::Context;
use crate::installations::InstalledProduct;
use anyhow::Result;
use clap::Args;
use console::style;
use serde::Serialize;

#[derive(Args)]
pub struct ListCmd {

}

#[derive(Serialize)]
struct ProductEntry {
    name: &'static str,
    version: String,
    ready: bool,
    path: PathBuf,
}

impl ListCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let launcher = ctx
            .installer()?
            .version()
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let entries: Vec<ProductEntry> = ctx.products.iter().map(product_entry).collect();

        println!(
            "{} {}  {}",
            style("Launcher").bold().cyan(),
            style(&launcher).bold(),
            style(ctx.installer()?.path().display()).dim(),
        );
        println!("{}", style("\nInstalled Products").bold());
        if entries.is_empty() {
            println!("  {}", style("(none)").dim());
            return Ok(());
        }

        for e in &entries {
            let ready = if e.ready {
                style("ready").green()
            } else {
                style("not ready").red()
            };

            println!(
                "  {} {}  {} {} {}",
                style("•").dim(),
                style(e.name).bold().cyan(),
                style(&e.version).bold(),
                style(e.path.display()).dim(),
                ready,
            );
        }
        Ok(())
    }
}

fn product_entry(p: &InstalledProduct) -> ProductEntry {
    match p {
        InstalledProduct::Houdini(h) => ProductEntry {
            name: "Houdini",
            version: h.version.to_string(),
            ready: h.ready,
            path: h.path.clone()
        },
        InstalledProduct::HServer(i) => ProductEntry {
            name: "HServer",
            version: i.version.to_string(),
            ready: i.ready,
            path: i.path.clone()
        },
        InstalledProduct::LicenseServer(i) => ProductEntry {
            name: "License Server",
            version: i.version.to_string(),
            ready: i.ready,
            path: i.path.clone()
        },
        InstalledProduct::HQueueServer(i) => ProductEntry {
            name: "HQueue Server",
            version: i.version.to_string(),
            ready: i.ready,
            path: i.path.clone()
        },
        InstalledProduct::HQueueClient(i) => ProductEntry {
            name: "HQueue Client",
            version: i.version.to_string(),
            ready: i.ready,
            path: i.path.clone()
        },
    }
}
