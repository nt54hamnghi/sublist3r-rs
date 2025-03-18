use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use reqwest::{Client, Response, header};

use super::{DEFAULT_USER_AGENT, Extract, Pagination, Search, Settings};

const API_URL: &str = "https://api.dnsdumpster.com/htmld/";
const SETTINGS: Settings = Settings {
    name: "DNSDumpster",
    base_url: "https://dnsdumpster.com",
    user_agent: DEFAULT_USER_AGENT,
    max_rounds: 1,
};

static INIT_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"hx-headers='\{"Authorization":\s*"(?<token>[[:alnum:]_\.-]+)"\}'"#)
        .expect("failed to compile regex for DNSDumpster init token")
});

#[derive(Extract)]
#[extract(pattern = r#"<td>(?<subdomain>.*?\.{domain})<\/td>"#)]
pub struct DNSDumpster {
    #[extract(domain)]
    domain: String,
}

impl DNSDumpster {
    pub fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
        }
    }

    /// Initializes the DNSDumpster session and returns the authorization token
    async fn init(&self, client: Client) -> Result<Option<String>, reqwest::Error> {
        let init = client
            .get(SETTINGS.base_url)
            .header(header::REFERER, "https://dnsdumpster.com")
            .send()
            .await?;

        let body = init.text().await?;

        let token = INIT_TOKEN_RE.captures(&body).map(|c| c["token"].to_owned());

        Ok(token)
    }
}

impl Pagination for DNSDumpster {
    /// `DNSDumpster` only runs once, no need to delay
    async fn delay(&self) {}
}

impl Search for DNSDumpster {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        self.domain.clone()
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
        // default token to an empty string,
        // which will cause 401 Unauthorized when the post request is made
        let token = self.init(client.clone()).await?.unwrap_or_default();

        client
            .post(API_URL)
            .form(&[("target", &self.domain)])
            .header(header::ACCEPT, "text/html")
            .header(header::AUTHORIZATION, token)
            .header("HX-Current-URL", SETTINGS.base_url)
            .header("HX-Request", "true")
            .header("HX-Target", "results")
            .header(header::ORIGIN, SETTINGS.base_url)
            .header(header::REFERER, SETTINGS.base_url)
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .send()
            .await
    }
}
