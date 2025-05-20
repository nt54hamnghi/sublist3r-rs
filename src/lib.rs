use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use enumerate::{Engine, EngineChoice, Enumerator, defaults_headers};
use reqwest::Client;
use strum::VariantArray;

pub mod cli;
mod enumerate;

#[tracing::instrument(skip_all)]
pub async fn run(domain: &str, choices: Vec<EngineChoice>) -> anyhow::Result<()> {
    let client = Client::builder()
        .default_headers(defaults_headers())
        .cookie_store(true)
        .gzip(true) // enable gzip compression
        .build()?;

    let engines: Vec<Engine> = if choices.is_empty() {
        Engine::from_iter(EngineChoice::VARIANTS.to_owned(), domain)
    } else {
        Engine::from_iter(choices, domain)
    };

    let subdomains = Arc::new(Mutex::new(HashSet::<String>::new()));

    let mut join_set = tokio::task::JoinSet::new();
    for ng in engines {
        let r = subdomains.clone();
        let c = client.clone();
        join_set.spawn(async move {
            let e = Enumerator::new(ng);
            e.print_banner();
            let found = e.enumerate(c).await;
            let mut guard = r.lock().unwrap();
            guard.extend(found.into_iter());
        });
    }

    join_set.join_all().await;

    println!();
    for sub in subdomains.lock().unwrap().iter() {
        println!("{sub}");
    }

    Ok(())
}
