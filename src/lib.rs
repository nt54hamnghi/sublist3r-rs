#![allow(unused)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use enumerate::{Baidu, Bing, DNSDumpster, Engine, Enumerator, Google, VirusTotal, Yahoo,
                defaults_headers};
use reqwest::Client;
use tracing::info;

mod enumerate;

#[tracing::instrument(skip_all)]
pub async fn run(_input: &str) -> anyhow::Result<()> {
    enumerate().await
}

async fn test_run() -> anyhow::Result<()> {
    Ok(())
}

async fn enumerate() -> anyhow::Result<()> {
    info!("initializing client...");
    let client = Client::builder()
        .default_headers(defaults_headers())
        .cookie_store(true)
        .gzip(true) // enable gzip compression
        .build()?;

    let domain = "***REMOVED***";
    let engines: Vec<Engine> = vec![
        // Google::new(domain).into(),
        // Yahoo::new(domain).into(),
        // Bing::new(domain).into(),
        // Baidu::new(domain).into(),
        DNSDumpster::new(domain).into(),
        VirusTotal::new(domain).into(),
    ];
    let subdomains = Arc::new(Mutex::new(HashSet::<String>::new()));

    let mut join_set = tokio::task::JoinSet::new();
    for ng in engines {
        let r = subdomains.clone();
        let c = client.clone();
        join_set.spawn(async move {
            let e = Enumerator::new(ng);
            let found = e.enumerate(c).await;
            let mut guard = r.lock().unwrap();
            guard.extend(found.into_iter());
        });
    }

    let output = join_set.join_all().await;

    for sub in subdomains.lock().unwrap().iter() {
        println!("{}", sub);
    }

    Ok(())
}
