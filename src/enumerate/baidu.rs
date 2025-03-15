use std::collections::HashSet;

use reqwest::{Client, Response, header};

use super::{DEFAULT_USER_AGENT, Extract, Search, Settings};

const PER_PAGE: usize = 10;
const SETTINGS: Settings = Settings {
    name: "Baidu",
    base_url: "https://www.baidu.com/s",
    user_agent: DEFAULT_USER_AGENT,
    max_pages: 20,
};

#[derive(Extract)]
#[extract(
    pattern = r#"<span class="c-color-gray" aria-hidden="true">(?<subdomain>.*?\.{domain})\/<\/span>"#
)]
pub(crate) struct Baidu {
    #[extract(domain)]
    domain: String,
}

impl Baidu {
    pub(crate) fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
        }
    }
}

impl Search for Baidu {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        // TODO: consider limiting the number of subdomains to exclude
        let found = subdomains
            .iter()
            .fold(String::new(), |acc, d| format!("{} -site:{}", acc, d));

        format!("site:{0} -site:www.{0}{1}", self.domain, found)
    }

    fn settings(&self) -> Settings {
        SETTINGS
    }

    async fn search(
        &self,
        client: Client,
        query: &str,
        page: usize,
    ) -> Result<Response, reqwest::Error> {
        let pn = page * PER_PAGE;

        client
            .get(SETTINGS.base_url)
            .query(&[("wq", query)]) // query
            .query(&[("pn", pn)]) // page number
            .query(&[("oq", query)]) // original query
            .query(&[("ie", "utf-8")]) // encoding
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .send()
            .await
    }
}
