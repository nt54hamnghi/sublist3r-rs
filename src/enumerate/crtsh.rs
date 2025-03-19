use std::borrow::Cow;
use std::collections::HashSet;

use reqwest::{Client, Response, header};
use serde::Deserialize;

use super::{DEFAULT_USER_AGENT, Extract, Search, Settings};

const SETTINGS: Settings = Settings {
    name: "CrtSh",
    base_url: "https://crt.sh/json",
    user_agent: DEFAULT_USER_AGENT,
    max_rounds: 1,
};

pub struct CrtSh {
    domain: String,
}

impl CrtSh {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
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

impl Search for CrtSh {
    fn settings(&self) -> Settings {
        SETTINGS
    }

    fn next_query(&self, _: &HashSet<String>) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(&self.domain))
    }

    async fn search(&self, client: Client, _: &str, _: usize) -> Result<Response, reqwest::Error> {
        client
            .get(SETTINGS.base_url)
            .query(&[("q", &self.domain)])
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .send()
            .await
    }

    /// `CrtSh` only runs once, no need to delay
    async fn delay(&self) {}
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
