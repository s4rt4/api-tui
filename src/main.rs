use anyhow::Result;
use apitester::{config::Cli, headless, run};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    tracing::info!(?cli, "starting apitester");

    if let Some(name) = cli.headless.clone() {
        let code = headless::run(&cli, &name).await?;
        std::process::exit(code);
    }

    run::run_tui(cli).await
}

fn install_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        hook(info);
    }));
}
