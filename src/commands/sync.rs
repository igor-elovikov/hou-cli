use crate::commands::package::print_sync_report;
use crate::installations::HoudiniInstallation;
use crate::package::Packages;
use crate::pip::{Pip, PipSyncStatus};
use crate::project::Project;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct SyncCmd {
    /// Skip patching package JSON files of re-fetched git packages.
    #[arg(long)]
    pub no_patch: bool,
}

impl SyncCmd {
    pub fn run(self, houdini: &HoudiniInstallation, project: &Project) -> Result<()> {
        let report = Packages::open_project(houdini, project, self.no_patch)?.sync()?;
        print_sync_report(&report);

        if project.manifest.pip.is_some() {
            match Pip::open(houdini, project)?.sync()? {
                Some(PipSyncStatus::Ok(n)) => println!("  ok      pip ({n} packages)"),
                Some(PipSyncStatus::Repaired(n)) => println!("  repair  pip ({n} packages)"),
                None => {}
            }
        }
        Ok(())
    }
}
