use std::collections::HashSet;
use std::time::Duration;

use enum_dispatch::enum_dispatch;
pub(crate) use enumerate_derive::Extract;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use reqwest::{Client, Response};
use tracing::{info, trace, warn};

pub(crate) use self::bing::Bing;
pub(crate) use self::google::Google;
pub(crate) use self::yahoo::Yahoo;

pub(crate) mod bing;
pub(crate) mod google;
pub(crate) mod yahoo;

const MAX_RETRIES: usize = 8;
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

/// regex pattern for a subdomain
/// It ensures proper domain name format:
///  1. One or more labels separated by dots
///  2. Each label can contain alphanumeric characters and hyphens
///  3. Hyphens can appear in the middle, but not at start/end
///  4. No empty labels (consecutive dots)
const SUBDOMAIN_RE_STR: &str = r#"(?:[[:alnum:]](?:[[:alnum:]-]*[[:alnum:]])?)(?:\.[[:alnum:]](?:[[:alnum:]-]*[[:alnum:]])?)*"#;

pub(crate) fn defaults_headers() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(3);

    // advertises that this client can handle `text/html`, `application/xhtml+xml`, and `application/xml`, etc.,
    // with a preference for `text/html` and `application/xhtml+xml`.
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        ),
    );
    // advertises that this client understands US English.
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.8"));
    // advertises that this client can handle gzip-compressed responses.
    // this is to potentially save bandwidth and speed up the response time.
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));

    headers
}

#[enum_dispatch(Extract, Search)]
pub(crate) enum Engine {
    Google,
    Yahoo,
    Bing,
}

#[enum_dispatch]
pub(crate) trait Extract {
    fn extract(&self, input: &str) -> HashSet<String>;
}

/// Settings for a Search Engine
pub(crate) struct Settings {
    name: &'static str,
    base_url: &'static str,
    user_agent: &'static str,
    max_pages: usize,
}

#[enum_dispatch]
pub(crate) trait Search {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String;

    fn settings(&self) -> Settings;

    async fn search(
        &self,
        client: Client,
        query: &str,
        page: usize,
    ) -> Result<Response, reqwest::Error>;
}

pub(crate) struct Enumerator<E> {
    engine: E,
}

impl<E> Enumerator<E>
where
    E: Search + Extract,
{
    pub fn new(engine: E) -> Self {
        Self { engine }
    }
}

impl<E> Enumerator<E>
where
    E: Search + Extract,
{
    #[tracing::instrument(skip_all, fields(NAME))]
    pub async fn enumerate(self, client: Client) -> HashSet<String> {
        let mut page = 0;
        let mut retries = 0;
        let mut found = 0;
        let mut subdomains = HashSet::new();

        #[allow(non_snake_case)]
        let Settings {
            name: NAME,
            max_pages: MAX_PAGES,
            ..
        } = self.engine.settings();

        // Record the name as part of the current span.
        tracing::Span::current().record("NAME", NAME);

        loop {
            trace!(page, found, retries, "searching");
            if retries > MAX_RETRIES || page > MAX_PAGES {
                info!(retries, page, "completed");
                break;
            }

            let query = self.engine.generate_query(&subdomains);

            let resp = match self
                .engine
                .search(client.clone(), &query, page)
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(err = ?e, "failed to search");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    retries += 1;
                    continue;
                }
            };

            info!(url = resp.url().to_string(), "searching");

            let body = match resp.text().await {
                Ok(b) => b,
                Err(e) => {
                    warn!(err = ?e, "failed to parse search results");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    retries += 1;
                    continue;
                }
            };

            // informs the executor that this task is about to block the thread
            // so any other tasks can be moved to a new worker thread
            tokio::task::block_in_place(|| {
                subdomains.extend(self.engine.extract(&body));
            });

            if found != subdomains.len() {
                found = subdomains.len();
                // subtracts 1 and saturates at 0 instead of underflowing if the result would be negative.
                retries = retries.saturating_sub(1);
            } else {
                retries += 1;
            }
            page += 1;

            // Sleep after each page to avoid being blocked
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        subdomains
    }
}
