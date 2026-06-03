pub use clap::{Parser, Subcommand};

pub mod eula;
pub mod init;
pub mod install;
pub mod list;
pub mod login;
pub mod logout;
mod package;
mod run;
mod sidefx;
pub mod setup;
pub mod uninstall;
pub mod update;

#[derive(Subcommand)]
pub enum Commands {
    Setup,
    /// Update the SideFX launcher to the latest production build.
    Update,
    Init(init::InitCmd),
    Run(run::Run),
    Sidefx(sidefx::SideFX),
    Package(package::PackageCmd),
    /// Install a Houdini build via the discovered installer.
    Install(install::InstallCmd),
    /// Uninstall an installed Houdini build.
    Uninstall(uninstall::UninstallCmd),
    /// List installed Houdini products.
    List(list::ListCmd),
    /// Store SideFX credentials in the config-dir credentials.toml.
    Login(login::LoginCmd),
    /// Remove the SideFX credentials/EULA settings file.
    Logout(logout::LogoutCmd),
    /// Manage accepted SideFX EULA dates.
    Eula(eula::EulaCmd),
}

#[derive(Parser)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    #[arg(short, long)]
    pub version: Option<String>,
    /// Keep stdio attached to the terminal and wait for Houdini to exit.
    #[arg(short, long)]
    pub attach: bool,
    /// Optional file (e.g. a .hip file) or project directory to open.
    pub file: Option<String>,
    /// Arguments forwarded to Houdini; everything after `--`.
    #[arg(last = true)]
    pub houdini_args: Vec<String>,
}
