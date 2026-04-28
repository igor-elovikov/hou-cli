use crate::hou::Context;
use crate::installations::HoudiniInstallation;
use crate::package::manifest::SourceMetadata;
use crate::package::source::InstallSpec;
use crate::package::update::UpdateTarget;
use crate::package::{CheckReport, Packages};
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct PackageCmd {
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
    /// Verify package integrity via stored checksums.
    Check(CheckArgs),
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

#[derive(Args)]
pub struct CheckArgs {
    /// Re-install packages whose checksums don't match.
    #[arg(long)]
    pub repair: bool,
}

impl PackageCmd {
    pub fn run(self, ctx: &Context, houdini: &HoudiniInstallation) -> Result<()> {
        let mut pkgs = Packages::open(ctx, houdini)?;
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
            PackageAction::Check(a) => {
                let report = pkgs.check(a.repair)?;
                print_report(&report);
                Ok(())
            }
        }
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

fn print_report(report: &CheckReport) {
    for p in &report.ok {
        println!("  ok      {}", p.display());
    }
    for p in &report.mismatched {
        println!("  bad     {}", p.display());
    }
    for p in &report.missing {
        println!("  missing {}", p.display());
    }
    for p in &report.repaired {
        println!("  repair  {}", p.display());
    }
    for p in &report.skipped {
        println!("  skip    {}", p.display());
    }
}
