use crate::commands::Commands;
use anyhow::Result;
use clap::Parser;
use commands::Cli;

mod commands;
mod hou;
mod installer;
mod installations;
mod sidefx;

pub fn main() -> Result<()> {
    let hou = hou::Context::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.latest_houdini()?;
            cmd.run(houdini)?;
        },
        Some(Commands::Sidefx(cmd)) => {
            cmd.run()?;
        },
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_builds_list() {
        let client = sidefx::Client::new().expect("client init");
        let builds = client
            .builds(sidefx::Product::Houdini(sidefx::Houdini::Qt6))
            .platform(sidefx::Platform::Linux)
            .only_production()
            .send()
            .expect("fetch builds");

        assert!(!builds.is_empty());
        for b in builds.iter().take(5) {
            println!("{:#?}", b);
        }
    }

    #[test]
    fn daily_build_download() {
        let client = sidefx::Client::new().expect("client init");
        let info = client
            .build_download(
                sidefx::Product::HoudiniLauncher(sidefx::HoudiniLauncher::Default),
                "21.0",
                sidefx::BuildSpec::Production,
            )
            .platform(sidefx::Platform::Linux)
            .send()
            .expect("fetch download");

        println!("{:#?}", info);
        assert!(info.download_url.starts_with("http"));
    }

    #[test]
    fn download_launcher_to_data_dir() {
        let hou = hou::Context::new().expect("context");
        let client = sidefx::Client::new().expect("client");
        let info = client
            .build_download(
                sidefx::Product::HoudiniLauncher(sidefx::HoudiniLauncher::Default),
                "21.0",
                sidefx::BuildSpec::Production,
            )
            .platform(sidefx::Platform::Linux)
            .send()
            .expect("build download info");
        let path = client
            .download_build(&info, &hou.data_dir)
            .expect("download");
        println!("launcher downloaded to {}", path.display());
        assert!(path.exists());
    }

    #[test]
    #[ignore]
    fn install_launcher_to_data_dir() {
        let hou = hou::Context::new().expect("context");
        let client = sidefx::Client::new().expect("client");
        let path = client
            .install_launcher(sidefx::HoudiniLauncher::Default, "21.0", &hou.data_dir)
            .expect("install launcher");
        println!("launcher installed at {}", path.display());
        assert!(path.exists());
    }
}
