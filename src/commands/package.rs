use crate::hou::Context;
use crate::installations::HoudiniInstallation;
use crate::package::manifest::{Manifest, SourceMetadata};
use crate::package::source::InstallSpec;
use crate::package::update::UpdateTarget;
use crate::package::{Packages, ScopeKind, SyncReport};
use crate::project::Project;
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use console::style;
use std::path::Path;

#[derive(Parser)]
pub struct PackageCmd {
    /// Operate on the global manifest, even when inside a project.
    #[arg(long, global = true, conflicts_with = "local")]
    pub global: bool,
    /// Operate on the project manifest (requires being inside a project).
    #[arg(long, global = true)]
    pub local: bool,
    /// Houdini version filter (inside a project requires --global).
    #[arg(short, long, global = true)]
    pub version: Option<String>,
    /// Skip patching package JSON files after install/update/sync.
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
    /// Override the package name used for the installation directory.
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
        if matches!(self.action, PackageAction::List) {
            return self.list(ctx, houdini, project);
        }

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
            PackageAction::List => unreachable!("handled above"),
            PackageAction::Sync => {
                let report = pkgs.sync()?;
                print_sync_report(&report);
                Ok(())
            }
        }
    }

    fn list(
        self,
        ctx: &Context,
        houdini: &HoudiniInstallation,
        project: Option<&Project>,
    ) -> Result<()> {
        if self.local && project.is_none() {
            bail!("--local requires being inside a project");
        }
        let show_global = !self.local;
        let show_project = !self.global && project.is_some();
        let mut wrote = false;

        if show_project {
            let p = project.unwrap();
            let pkgs = Packages::open_project(houdini, p, self.no_patch)?;
            print_project_header(p);
            print_packages(pkgs.list());
            wrote = true;
        }

        if show_global {
            // outside a project with no filter: every installed major.minor line
            let targets: Vec<&HoudiniInstallation> = if project.is_none() && self.version.is_none()
            {
                let mut hs: Vec<_> = ctx.houdinis().collect();
                hs.sort_by(|a, b| b.version.cmp(&a.version));
                hs.dedup_by_key(|h| (h.version.major, h.version.minor));
                hs
            } else {
                vec![houdini]
            };
            for h in targets {
                if wrote {
                    println!();
                }
                let pkgs = Packages::open_global(ctx, h, self.no_patch)?;
                print_global_header(h);
                print_packages(pkgs.list());
                wrote = true;
            }
        }
        Ok(())
    }
}

fn print_global_header(houdini: &HoudiniInstallation) {
    let line = format!("{}.{}", houdini.version.major, houdini.version.minor);
    println!("{} {}", style("Global").bold(), style(line).bold().cyan());
}

fn print_project_header(project: &Project) {
    let name = project
        .root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?");
    let label = style("Project").bold();
    let name_styled = style(name).bold().cyan();
    if project.isolated() {
        println!("{label} {name_styled}  {}", style("(isolated)").yellow());
    } else {
        println!("{label} {name_styled}");
    }
}

fn print_packages(manifest: &Manifest) {
    if manifest.hou_package_manifest.is_empty() {
        println!("  {}", style("(no packages)").dim());
        return;
    }
    for (path, entry) in &manifest.hou_package_manifest {
        let name = display_name(path, entry);
        let detail = match entry {
            SourceMetadata::Git(g) => format!("{} @ {}", g.url, g.version),
            SourceMetadata::Folder(f) => f.path.display().to_string(),
        };
        println!(
            "  {} {}  {}",
            style("•").dim(),
            style(&name).bold().cyan(),
            style(&detail).dim()
        );
    }
}

fn display_name(path: &Path, entry: &SourceMetadata) -> String {
    let base = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    match entry {
        SourceMetadata::Git(_) => strip_hash_suffix(base).to_string(),
        SourceMetadata::Folder(_) => base.to_string(),
    }
}

fn strip_hash_suffix(s: &str) -> &str {
    if let Some((name, hash)) = s.rsplit_once('-') {
        if hash.len() == 8
            && hash
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return name;
        }
    }
    s
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
