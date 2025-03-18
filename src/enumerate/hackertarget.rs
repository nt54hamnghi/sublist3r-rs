use std::collections::HashSet;
use std::hash::Hash;

use reqwest::{Client, Response, header};
use serde::Deserialize;

use super::{DEFAULT_USER_AGENT, Extract, Pagination, Search, Settings};

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

impl Pagination for HackerTarget {
    /// `HackerTarget` only runs once, no need to delay
    async fn delay(&self) {}
}

impl Search for HackerTarget {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        self.domain.to_owned()
    }

    fn settings(&self) -> Settings {
        SETTINGS
    }

    async fn search(
        &mut self,
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
}
