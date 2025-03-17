use clap::CommandFactory;
use owo_colors::OwoColorize;
use sublist3r_rs::prelude::*;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        domain,
        engines,
        verbose,
        completion,
    } = Cli::parse();

    if let Some(shell) = completion {
        print_completions(shell, &mut Cli::command());
        return Ok(());
    }

    let level = if verbose { Level::INFO } else { Level::WARN };

    // build a formatting subscriber with a max level of WARN
    tracing_subscriber::fmt().with_max_level(level).init();

    // domain is None only if completion is provided
    // which is already handled above, so we can safely unwrap
    let domain = domain.unwrap();
    let domain = domain
        .domain()
        .ok_or_else(|| anyhow::anyhow!("Invalid domain"))?;

    println!("{}", header());
    println!(
        "{} {}",
        "[-] Enumerating subdomains now for".blue(),
        domain.blue()
    );

    run(domain, engines).await?;

    Ok(())
}
