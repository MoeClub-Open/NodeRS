mod accounting;
mod acme;
mod config;
mod logging;
mod management;
mod panel;
mod protocols;
mod pxboard;
mod runtime;
mod status;
mod ws;

use anyhow::Context;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let argument = std::env::args().nth(1);
    if argument.as_deref() == Some("--supported-panels") {
        println!("xboard pxboard");
        return Ok(());
    }
    if matches!(argument.as_deref(), Some("--version" | "-V")) {
        println!("noders {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let _ = rustls::crypto::ring::default_provider().install_default();
    logging::init();

    if argument.as_deref() == Some("--migrate-panel") {
        let arguments: Vec<_> = std::env::args().skip(2).collect();
        anyhow::ensure!(
            arguments.len() == 2,
            "usage: noders --migrate-panel <existing-config.toml> <target-config.toml>"
        );
        return management::migrate_file(
            std::path::Path::new(&arguments[0]),
            std::path::Path::new(&arguments[1]),
        )
        .await;
    }

    let config_path = argument
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.toml"));

    let config = config::AppConfig::load(&config_path)
        .await
        .with_context(|| format!("load config from {}", config_path.display()))?;

    management::run(&config_path, config).await
}
