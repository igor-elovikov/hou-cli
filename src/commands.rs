use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};

pub mod config;
pub mod eula;
pub mod find;
pub mod init;
pub mod install;
pub mod launcher;
pub mod list;
pub mod login;
pub mod logout;
mod package;
mod run;
mod sidefx;
pub mod uninstall;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Subcommand)]
pub enum Commands {
    Launcher(launcher::LauncherCmd),
    /// List installed Houdini products.
    #[command(visible_alias = "ls")]
    List(list::ListCmd),
    /// Find Houdini products available for download.
    #[command(visible_alias = "f")]
    Find(find::FindCmd),
    /// Install a Houdini product via the discovered installer.
    #[command(visible_alias = "i")]
    Install(install::InstallCmd),
    /// Uninstall an installed Houdini product.
    #[command(visible_alias = "rm")]
    Uninstall(uninstall::UninstallCmd),
    /// Store SideFX credentials in the config-dir credentials.toml.
    Login(login::LoginCmd),
    /// Remove the SideFX credentials/EULA settings file.
    Logout(logout::LogoutCmd),
    /// Manage accepted SideFX EULA dates.
    Eula(eula::EulaCmd),
    /// Run anything from Houdini environment
    #[command(visible_alias = "x")]
    Run(run::Run),
    /// Package management
    #[command(visible_alias = "pm")]
    Package(package::PackageCmd),
    /// Initialize project in directory
    Init(init::InitCmd),
    /// Calls to SideFX WebAPI. Builds list, downloading and changelog
    Sidefx(sidefx::SideFX),
    /// Manage configuration settings for hou.
    Config(config::ConfigCmd),
}

#[derive(Parser)]
#[command(styles = STYLES, args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,

    /// Houdini version to run command against
    #[arg(short, long, global = true)]
    pub version: Option<String>,

    /// Keep stdio attached to the terminal and wait for Houdini to exit.
    #[arg(short, long)]
    pub attach: bool,

    /// Optional file (e.g. a .hip file) or project directory to open.
    #[arg(value_name = "File or Project Directory")]
    pub file: Option<String>,

    /// Arguments forwarded to Houdini; everything after `--`.
    #[arg(last = true, value_name = "Houdini arguments")]
    pub houdini_args: Vec<String>,
}
