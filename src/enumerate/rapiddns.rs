use std::borrow::Cow;
use std::collections::HashSet;

use reqwest::{Client, Response, header};

use super::{Extract, Search, Settings};

const SETTINGS: Settings = Settings {
    name: "RapidDNS",
    base_url: "https://rapiddns.io/subdomain",
    user_agent: "",
    max_rounds: 10,
};

#[derive(Extract)]
#[extract(pattern = r#"<td>(?<subdomain>.*?\.{domain})<\/td>"#)]
pub struct RapidDNS {
    #[extract(domain)]
    domain: String,
}

impl RapidDNS {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

impl Search for RapidDNS {
    fn settings(&self) -> Settings {
        SETTINGS
    }

    fn next_query(&self, subdomains: &HashSet<String>) -> Option<Cow<'_, str>> {
        let query = format!("{}/{}", SETTINGS.base_url, self.domain);
        Some(Cow::Owned(query))
    }

    #[doc = " Search for a query on a page"]
    async fn search(
        &self,
        client: Client,
        url: &str,
        page: usize,
    ) -> Result<Response, reqwest::Error> {
        client
            .get(url)
            .query(&[("page", page)])
            .query(&[("full", 1)])
            .header(header::ACCEPT, "text/html")
            .send()
            .await
    }

    /// `DNSDumpster` only runs once, no need to delay
    async fn delay(&self) {}
}
