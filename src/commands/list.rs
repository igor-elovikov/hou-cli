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
}

impl ListCmd {
    pub fn run(self, ctx: &Context) -> Result<()> {
        let entries: Vec<ProductEntry> = ctx.products.iter().map(product_entry).collect();

        if self.json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
            return Ok(());
        }

        println!("{}", style("Installed Products").bold());
        if entries.is_empty() {
            println!("  {}", style("(none)").dim());
            return Ok(());
        }

        for e in &entries {
            println!(
                "  {} {}  {}",
                style("•").dim(),
                style(e.name).bold().cyan(),
                &e.version
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
        },
        InstalledProduct::HServer(i) => ProductEntry {
            name: "HServer",
            version: i.version.to_string(),
        },
        InstalledProduct::LicenseServer(i) => ProductEntry {
            name: "License Server",
            version: i.version.to_string(),
        },
        InstalledProduct::HQueueServer(i) => ProductEntry {
            name: "HQueue Server",
            version: i.version.to_string(),
        },
        InstalledProduct::HQueueClient(i) => ProductEntry {
            name: "HQueue Client",
            version: i.version.to_string(),
        },
    }
}
