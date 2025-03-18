use std::collections::HashSet;
use std::hash::Hash;

use reqwest::{Client, Response, header};
use serde::Deserialize;

use super::{DEFAULT_USER_AGENT, Extract, Pagination, Search, Settings};

const SETTINGS: Settings = Settings {
    name: "AlienVault",
    base_url: "https://otx.alienvault.com/api/v1/indicators/domain",
    user_agent: "", // not used
    max_rounds: 1,
};

pub struct AlienVault {
    domain: String,
}

impl AlienVault {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

impl Extract for AlienVault {
    fn extract(&mut self, input: &str) -> HashSet<String> {
        serde_json::from_str::<AlienVaultResponse>(input)
            .map(|r| r.data)
            .unwrap_or_default()
    }
}

impl Pagination for AlienVault {
    /// `AlienVault` only runs once, no need to delay
    async fn delay(&self) {}
}

impl Search for AlienVault {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        let domain = &self.domain;
        let base_url = SETTINGS.base_url;
        format!("{base_url}/{domain}/passive_dns")
    }

    fn settings(&self) -> Settings {
        SETTINGS
    }

    async fn search(
        &mut self,
        client: Client,
        url: &str,
        _: usize,
    ) -> Result<Response, reqwest::Error> {
        client.get(url).send().await
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "PassiveDns")]
struct AlienVaultResponse {
    data: HashSet<String>,
}

impl From<PassiveDns> for AlienVaultResponse {
    fn from(value: PassiveDns) -> Self {
        let PassiveDns { passive_dns, count } = value;

        let mut data = HashSet::with_capacity(count as usize);
        let v = passive_dns.into_iter().map(|i| i.hostname);
        data.extend(v);

        Self { data }
    }
}

#[derive(Debug, Deserialize)]
struct PassiveDns {
    passive_dns: Vec<Item>,
    count: i64,
}

#[derive(Debug, Deserialize)]
struct Item {
    hostname: String,
}
