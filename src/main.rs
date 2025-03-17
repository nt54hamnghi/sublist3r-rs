use sublist3r_rs::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt::init();

    let domain = args
        .domain
        .domain()
        .ok_or_else(|| anyhow::anyhow!("Invalid domain"))?;

    run(domain, args.engines).await?;

    Ok(())
}
