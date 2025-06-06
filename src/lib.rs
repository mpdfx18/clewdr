use std::{path::PathBuf, sync::LazyLock};

use clap::Parser;
use const_format::formatc;
use figlet_rs::FIGfont;

pub mod api;
pub mod bootstrap;
pub mod config;
pub mod error;
pub mod router;
pub mod services;
pub mod state;
pub mod types;
pub mod utils;

pub const IS_DEV: bool = cfg!(debug_assertions);

pub const VERSION_INFO: &str = formatc!(
    "v{} by {} in {} mode",
    env!("CARGO_PKG_VERSION"),
    env!("CARGO_PKG_AUTHORS"),
    if IS_DEV { "dev" } else { "production" }
);

/// Header for the application
pub static BANNER: LazyLock<String> = LazyLock::new(|| {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("ClewdR");
    let banner = figure.unwrap().to_string();
    format!("{}\n{}", banner, VERSION_INFO)
});

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    /// Force update of the application
    pub update: bool,
    #[arg(short, long)]
    /// load cookie from file
    pub file: Option<PathBuf>,
}
