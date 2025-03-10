use enumerate::google::Google;
use enumerate::{Enumerator, defaults_headers};
use reqwest::Client;

mod enumerate;

pub async fn run(input: &str) -> anyhow::Result<()> {
    let client = Client::builder()
        .default_headers(defaults_headers())
        .gzip(true) // enable gzip compression
        .build()?;

    let mut e = Enumerator::new(Google::new("google.com"));
    let subdomains = e.enumerate(client.clone()).await?;

    for sub in subdomains {
        println!("{}", sub);
    }

    Ok(())
}
