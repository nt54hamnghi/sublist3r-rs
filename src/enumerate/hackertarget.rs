use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::Hash;

use reqwest::{Client, Response, header};
use serde::Deserialize;

use super::{Extract, Search, Settings};

const SETTINGS: Settings = Settings {
    name: "HackerTarget",
    base_url: "https://api.hackertarget.com/hostsearch/",
    user_agent: "", // not used
    max_rounds: 1,
};

pub struct HackerTarget {
    domain: String,
}

impl HackerTarget {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

impl Extract for HackerTarget {
    fn extract(&mut self, input: &str) -> HashSet<String> {
        input
            .split("\n")
            .filter_map(|l| l.split_once(','))
            .map(|(host, _)| host.to_owned())
            .collect()
    }
}

impl Search for HackerTarget {
    fn settings(&self) -> Settings {
        SETTINGS
    }

    fn next_query(&self, subdomains: &HashSet<String>) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(&self.domain))
    }

    async fn search(
        &self,
        client: Client,
        query: &str,
        _: usize,
    ) -> Result<Response, reqwest::Error> {
        client
            .get(SETTINGS.base_url)
            .query(&[("q", query)])
            .send()
            .await
    }

    /// `HackerTarget` only runs once, no need to delay
    async fn delay(&self) {}
}
