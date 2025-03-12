use enumerate::google::Google;
use enumerate::yahoo::Yahoo;
use enumerate::{Enumerator, defaults_headers};
use reqwest::Client;
use tracing::info;

mod enumerate;

#[allow(unused)]
#[tracing::instrument(skip_all)]
pub async fn run(_input: &str) -> anyhow::Result<()> {
    info!("initializing client...");
    let client = Client::builder()
        .default_headers(defaults_headers())
        .gzip(true) // enable gzip compression
        .build()?;

    let mut e = Enumerator::new(Yahoo::new("google.com"));
    let mut e = Enumerator::new(Google::new("google.com"));

    let subdomains = e.enumerate(client.clone()).await;

    for sub in subdomains {
        println!("{}", sub);
    }

    Ok(())
}
