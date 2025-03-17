use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use reqwest::{Client, Response, header};
use serde::{Deserialize, Deserializer};

use super::{DEFAULT_USER_AGENT, Extract, Pagination, Search, Settings};

const PER_PAGE: usize = 10;
const SETTINGS: Settings = Settings {
    name: "VirusTotal",
    // the complete url is:
    // https://www.virustotal.com/ui/domains/{domain}/relationships/subdomains
    base_url: "https://www.virustotal.com/ui/domains",
    user_agent: DEFAULT_USER_AGENT,
    max_pages: 10,
};

pub struct VirusTotal {
    domain: String,
    meta: Option<Meta>,
    page: usize,
}

impl VirusTotal {
    pub fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
            meta: None,
            page: 0,
        }
    }

    /// Port of the `computeAntiAbuseHeader` function from VirusTotal's frontend
    ///
    /// ```ignore
    /// function computeAntiAbuseHeader() {
    ///     const e = Date.now() / 1e3;
    ///     return btoa(`${( () => {
    ///         const e = 1e10 * (1 + Math.random() % 5e4);
    ///         return e < 50 ? "-1" : e.toFixed(0)
    ///     }
    ///     )()}-ZG9udCBiZSBldmls-${e}`)
    /// }
    /// ```
    fn compute_anti_abuse_header() -> String {
        let secs_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let random = || {
            let entropy = 1e10 * (1.0 + fastrand::f64() % 5e4);
            if entropy < 50.0 {
                -1
            } else {
                entropy.round() as i64
            }
        };

        let random = random();

        // ZG9udCBiZSBldmls is "dont be evil" in base64
        BASE64_STANDARD.encode(format!("{random}-ZG9udCBiZSBldmls-{secs_since_epoch}"))
    }
}

impl Extract for VirusTotal {
    fn extract(&mut self, input: &str) -> HashSet<String> {
        match serde_json::from_str::<VirusTotalResponse>(input) {
            Ok(r) => {
                self.meta = Some(r.meta);
                r.data
            }
            Err(_) => HashSet::new(),
        }
    }
}

impl Pagination for VirusTotal {
    async fn delay(&self) {
        let dur = Duration::from_secs(2);
        tokio::time::sleep(dur).await;
    }

    fn stop(&self) -> bool {
        if self.page >= SETTINGS.max_pages {
            return true;
        }
        match &self.meta {
            Some(m) => m.cursor.is_none(),
            None => false,
        }
    }
}

impl Search for VirusTotal {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        let domain = &self.domain;
        let base_url = SETTINGS.base_url;
        let base_query = format!("{base_url}/{domain}/relationships/subdomains");

        match &self.meta {
            Some(m) => match &m.cursor {
                Some(c) => format!("{base_query}?cursor={c}"),
                None => base_query,
            },
            None => base_query,
        }
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
        let resp = client
            .get(url)
            .query(&[("limit", PER_PAGE)])
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .header("accept", "application/json")
            .header("accept-ianguage", "en-US,en;q=0.9,es;q=0.8")
            .header("accept-language", "en-US,en;q=0.8")
            .header("x-app-version", "v1x356x0")
            .header("x-tool", "vt-ui-main")
            .header(
                "X-VT-Anti-Abuse-Header",
                VirusTotal::compute_anti_abuse_header(),
            )
            .send()
            .await?;

        if resp.status().is_success() {
            self.page += 1;
        }

        Ok(resp)
    }
}

#[derive(Debug, Deserialize)]
struct VirusTotalResponse {
    #[serde(deserialize_with = "VirusTotalResponse::deserialize_data")]
    pub data: HashSet<String>,
    pub meta: Meta,
}

impl VirusTotalResponse {
    fn deserialize_data<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Vec::<Domain>::deserialize(deserializer)?
            .into_iter()
            .map(|d| d.id)
            .collect::<HashSet<_>>();
        Ok(v)
    }
}

#[derive(Debug, Deserialize)]
struct Domain {
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct Meta {
    pub count: i64,
    pub cursor: Option<String>,
}
