use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::time::Duration;

/// Color theme for syntax highlighting in the response viewer.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn is_light(self) -> bool {
        matches!(self, Theme::Light)
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "apitester",
    version,
    about = "Interactive TUI for HTTP API testing"
)]
pub struct Cli {
    /// Path to collection file (.toml)
    pub collection: Option<PathBuf>,

    /// Environment to use for variable interpolation
    #[arg(short, long, default_value = "default")]
    pub env: String,

    /// Request timeout in seconds
    #[arg(short, long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,

    /// Skip TLS certificate verification
    #[arg(short = 'k', long)]
    pub insecure: bool,

    /// Disable following redirects
    #[arg(long)]
    pub no_redirect: bool,

    /// HTTP/HTTPS proxy URL
    #[arg(long)]
    pub proxy: Option<String>,

    /// Disable ANSI colors
    #[arg(long)]
    pub no_color: bool,

    /// Color theme for syntax highlighting
    #[arg(long, value_enum, default_value_t = Theme::Dark)]
    pub theme: Theme,

    /// Disable the persistent cookie jar
    #[arg(long)]
    pub no_cookies: bool,

    /// Run a single request non-interactively, print response, exit
    #[arg(long, value_name = "NAME")]
    pub headless: Option<String>,
}

impl Cli {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout)
    }
}
