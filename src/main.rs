use owo_colors::OwoColorize;
use sublist3r_rs::prelude::*;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let level = if args.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    // build a formatting subscriber with a max level of WARN
    tracing_subscriber::fmt().with_max_level(level).init();

    let domain = args
        .domain
        .domain()
        .ok_or_else(|| anyhow::anyhow!("Invalid domain"))?;

    println!("{}", header());
    println!(
        "{} {}",
        "[-] Enumerating subdomains now for".blue(),
        domain.blue()
    );

    run(domain, args.engines).await?;

    Ok(())
}
