use std::collections::HashSet;

use reqwest::{Client, Response, header};
use serde::Deserialize;

use super::{DEFAULT_USER_AGENT, Extract, Pagination, Search, Settings};

const SETTINGS: Settings = Settings {
    name: "CrtSh",
    base_url: "https://crt.sh/json",
    user_agent: DEFAULT_USER_AGENT,
    max_pages: 1,
};

pub struct CrtSh {
    domain: String,
    once: bool,
}

impl CrtSh {
    pub fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
            once: false,
        }
    }
}

impl Extract for CrtSh {
    fn extract(&mut self, input: &str) -> HashSet<String> {
        serde_json::from_str::<CrtShResponse>(input)
            .map(|r| r.data)
            .unwrap_or_default()
    }
}

impl Pagination for CrtSh {
    /// `CrtSh` only runs once, no need to delay
    async fn delay(&self) {}

    fn stop(&self) -> bool {
        self.once
    }
}

impl Search for CrtSh {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        self.domain.to_owned()
    }

    fn settings(&self) -> Settings {
        SETTINGS
    }

    async fn search(
        &mut self,
        client: Client,
        _: &str,
        _: usize,
    ) -> Result<Response, reqwest::Error> {
        let resp = client
            .get(SETTINGS.base_url)
            .query(&[("q", &self.domain)])
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .send()
            .await?;

        if resp.status().is_success() {
            self.once = true;
        }

        Ok(resp)
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "Vec<Item>")]
struct CrtShResponse {
    data: HashSet<String>,
}

impl From<Vec<Item>> for CrtShResponse {
    fn from(value: Vec<Item>) -> Self {
        let mut data = HashSet::with_capacity(value.len());
        for i in value {
            let domains = i.name_value.split("\n").map(String::from);
            data.extend(domains);
        }

        Self { data }
    }
}

#[derive(Debug, Deserialize)]
struct Item {
    name_value: String,
}
