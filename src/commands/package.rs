use crate::hou::Context;
use crate::installations::HoudiniInstallation;
use crate::package::manifest::SourceMetadata;
use crate::package::source::InstallSpec;
use crate::package::update::UpdateTarget;
use crate::package::{Packages, ScopeKind, SyncReport};
use crate::project::Project;
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct PackageCmd {
    /// Operate on the global manifest, even when inside a project.
    #[arg(long, global = true, conflicts_with = "local")]
    pub global: bool,
    /// Operate on the project manifest (requires being inside a project).
    #[arg(long, global = true)]
    pub local: bool,
    /// Skip patching package json files after install/update/sync.
    #[arg(long, global = true)]
    pub no_patch: bool,

    #[command(subcommand)]
    pub action: PackageAction,
}

#[derive(Subcommand)]
pub enum PackageAction {
    /// Install a package from a URL, local git repo, or folder.
    Install(InstallArgs),
    /// Remove a package by name or install path.
    Uninstall(UninstallArgs),
    /// Update a git package to a new version.
    Update(UpdateArgs),
    /// List installed packages.
    List,
    /// Re-fetch any git package whose cache dir is missing or has a checksum mismatch.
    Sync,
}

#[derive(Args)]
pub struct InstallArgs {
    /// URL, git repo path, or folder path.
    pub source: String,
    /// Override the package name used for the install directory.
    #[arg(long)]
    pub name: Option<String>,
    /// Specific tag (or raw commit) to install.
    #[arg(long, conflicts_with = "latest")]
    pub tag: Option<String>,
    /// Track HEAD instead of a tag.
    #[arg(long)]
    pub latest: bool,
}

#[derive(Args)]
pub struct UninstallArgs {
    pub name: String,
}

#[derive(Args)]
pub struct UpdateArgs {
    pub name: String,
    #[arg(long, conflicts_with = "latest")]
    pub tag: Option<String>,
    #[arg(long)]
    pub latest: bool,
}

impl PackageCmd {
    pub fn run(
        self,
        ctx: &Context,
        houdini: &HoudiniInstallation,
        project: Option<&Project>,
    ) -> Result<()> {
        let mut pkgs = match (self.global, self.local, project) {
            (true, _, _) => Packages::open_global(ctx, houdini, self.no_patch)?,
            (false, true, None) => bail!("--local requires being inside a project"),
            (false, true, Some(p)) => Packages::open_project(houdini, p, self.no_patch)?,
            (false, false, Some(p)) => Packages::open_project(houdini, p, self.no_patch)?,
            (false, false, None) => Packages::open_global(ctx, houdini, self.no_patch)?,
        };

        log::debug!(
            "Package scope: {} ({})",
            match pkgs.kind {
                ScopeKind::Global => "global",
                ScopeKind::Project => "project",
            },
            pkgs.houdini.version
        );

        match self.action {
            PackageAction::Install(a) => {
                let spec = InstallSpec::parse(&a.source, a.tag, a.latest, a.name)?;
                pkgs.install(spec)
            }
            PackageAction::Uninstall(a) => pkgs.uninstall(&a.name),
            PackageAction::Update(a) => {
                let target = match (a.tag, a.latest) {
                    (Some(_), true) => bail!("--tag and --latest are mutually exclusive"),
                    (Some(t), false) => UpdateTarget::Tag(t),
                    (None, true) => UpdateTarget::Latest,
                    (None, false) => UpdateTarget::Auto,
                };
                pkgs.update(&a.name, target)
            }
            PackageAction::List => {
                print_list(&pkgs);
                Ok(())
            }
            PackageAction::Sync => {
                let report = pkgs.sync()?;
                print_sync_report(&report);
                Ok(())
            }
        }
    }
}

fn print_sync_report(report: &SyncReport) {
    for p in &report.ok {
        println!("  ok      {}", p.display());
    }
    for p in &report.repaired {
        println!("  repair  {}", p.display());
    }
    for p in &report.skipped {
        println!("  skip    {}", p.display());
    }
}

fn print_list(pkgs: &Packages<'_>) {
    let m = pkgs.list();
    if m.hou_package_manifest.is_empty() {
        println!("No packages installed.");
        return;
    }
    for (path, entry) in &m.hou_package_manifest {
        let (kind, detail) = match entry {
            SourceMetadata::Git(g) => ("git", format!("{} @ {}", g.url, g.version)),
            SourceMetadata::Folder(_) => ("folder", String::new()),
        };
        println!("{:10} {}  [{}]", kind, path.display(), detail);
    }
}
