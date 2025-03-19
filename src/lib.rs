#![allow(unused)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use enumerate::{EngineChoice, defaults_headers};
use prelude::*;
use reqwest::Client;
use tracing::info;

pub mod cli;
pub mod enumerate;
pub mod prelude;

#[tracing::instrument(skip_all)]
pub async fn run(domain: &str, choices: Vec<EngineChoice>) -> anyhow::Result<()> {
    let client = Client::builder()
        .default_headers(defaults_headers())
        .cookie_store(true)
        .gzip(true) // enable gzip compression
        .build()?;

    let engines: Vec<Engine> = if choices.is_empty() {
        Engine::enum_vec(domain)
    } else {
        choices
            .into_iter()
            .map(|c| match c {
                EngineChoice::AlienVault => AlienVault::new(domain).into(),
                EngineChoice::Bing => Bing::new(domain).into(),
                EngineChoice::CrtSh => CrtSh::new(domain).into(),
                EngineChoice::DNSDumpster => DNSDumpster::new(domain).into(),
                EngineChoice::Google => Google::new(domain).into(),
                EngineChoice::HackerTarget => HackerTarget::new(domain).into(),
                EngineChoice::RapidDNS => RapidDNS::new(domain).into(),
                EngineChoice::VirusTotal => VirusTotal::new(domain).into(),
                EngineChoice::Yahoo => Yahoo::new(domain).into(),
            })
            .collect()
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

    let output = join_set.join_all().await;

    println!();
    for sub in subdomains.lock().unwrap().iter() {
        println!("{}", sub);
    }

    Ok(())
}
