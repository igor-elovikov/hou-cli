use crate::commands::Commands;
use anyhow::{Context, Result};
use clap::Parser;
use commands::Cli;
use reqwest::Client;
use serde::Deserialize;

mod commands;
mod hou;
mod installer;
mod products;

const CLIENT_ID: &str = "j6VpXfB18GrkBsvO1SPrr5Z2wxwjjbmS9QiuVGFN";
const CLIENT_SECRET: &str = "ymW6Zeenh5j2xCPtxB4RcDpMXkEDqAF9d0rEJETExiCyx1AqrKAaLFoZqUrXQGETibHtGzQMtGJ0CXKKocTd8bt43C7McEGQpKoJmdD62xg494Reo1HkjiV1btPg7C8S";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Todo {
    title: String,
    user_id: i32,
}

async fn fetch_todo(client: &Client) -> Result<Todo> {
    let url = "https://jsonplaceholder.typicode.com/todos/1";

    let todo = client
        .get(url)
        .send()
        .await
        .context("Failed to send GET request")? // Adds custom context to the error
        .error_for_status() // Converts 4xx/5xx HTTP responses into Errors
        .context("Server returned an error status")?
        .json()
        .await
        .context("Failed to parse JSON response")?;

    Ok(todo)
}

// #[tokio::main]
pub fn main() -> Result<()> {
    let hou = hou::Context::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run(cmd)) => {
            let houdini = hou.latest_houdini()?;
            cmd.run(houdini)?;
        }
        _ => {}
    }

    Ok(())
}
