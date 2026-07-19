use clap::{Parser, Subcommand};
use clap::builder::styling::{AnsiColor, Effects, Styles};

pub mod config;
pub mod eula;
pub mod init;
pub mod install;
pub mod list;
pub mod login;
pub mod logout;
mod package;
mod run;
mod sidefx;
pub mod uninstall;
pub mod launcher;
pub mod find;

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
    /// Install a Houdini build via the discovered installer.
    #[command(visible_alias = "i")]
    Install(install::InstallCmd),
    /// Uninstall an installed Houdini build.
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
    /// Read and write hou settings in the config-dir config.toml.
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
