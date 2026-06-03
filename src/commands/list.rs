use crate::hou::Context;
use crate::installations::InstalledProduct;
use anyhow::Result;
use clap::Args;
use console::style;
use serde::Serialize;

#[derive(Args)]
pub struct ListCmd {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct ProductEntry {
    name: &'static str,
    version: String,
    ready: bool,
}

#[derive(Serialize)]
struct ListOutput {
    launcher: String,
    products: Vec<ProductEntry>,
}

impl ListCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let launcher = ctx
            .installer
            .version()
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let entries: Vec<ProductEntry> = ctx.products.iter().map(product_entry).collect();

        if self.json {
            let output = ListOutput {
                launcher,
                products: entries,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        println!(
            "{} {}",
            style("Launcher").bold().cyan(),
            style(&launcher).bold(),
        );
        println!("{}", style("Installed Products").bold());
        if entries.is_empty() {
            println!("  {}", style("(none)").dim());
            return Ok(());
        }

        for e in &entries {
            let ready = if e.ready {
                style("Yes").green()
            } else {
                style("No").red()
            };
            println!(
                "  {} {}  {}  {}{}",
                style("•").dim(),
                style(e.name).bold().cyan(),
                &e.version,
                style("ready:").dim(),
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
            ready: h.ready(),
        },
        InstalledProduct::HServer(i) => ProductEntry {
            name: "HServer",
            version: i.version.to_string(),
            ready: i.ready,
        },
        InstalledProduct::LicenseServer(i) => ProductEntry {
            name: "License Server",
            version: i.version.to_string(),
            ready: i.ready,
        },
        InstalledProduct::HQueueServer(i) => ProductEntry {
            name: "HQueue Server",
            version: i.version.to_string(),
            ready: i.ready,
        },
        InstalledProduct::HQueueClient(i) => ProductEntry {
            name: "HQueue Client",
            version: i.version.to_string(),
            ready: i.ready,
        },
    }
}
