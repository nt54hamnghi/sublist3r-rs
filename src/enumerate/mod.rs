use std::cmp::max;
use std::collections::HashSet;

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use reqwest::{Client, Response};
use tracing::{info, trace, warn};

pub(crate) mod google;

const MAX_PAGES: usize = 20;
const MAX_RETRIES: usize = 5;

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

pub(crate) trait Extract {
    fn extract(&self, input: &str) -> impl Iterator<Item = String>;
}

pub(crate) trait Search {
    const NAME: &str;
    const BASE_URL: &str;

    fn generate_query(&self, subdomains: &HashSet<String>) -> String;

    async fn search(
        &self,
        client: Client,
        query: &str,
        page: usize,
    ) -> Result<Response, reqwest::Error>;
}

pub(crate) struct Enumerator<E> {
    engine: E,
    subdomains: HashSet<String>,
}

impl<E> Enumerator<E>
where
    E: Search + Extract,
{
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            subdomains: HashSet::new(),
        }
    }
}

impl<E> Enumerator<E>
where
    E: Search + Extract,
{
    #[tracing::instrument(skip_all, fields(name = E::NAME))]
    pub async fn enumerate(&mut self, client: Client) -> &HashSet<String> {
        let mut page = 0;
        let mut retries = 0;
        let mut found = 0;

        loop {
            trace!(page, found, retries, "searching");
            if retries > MAX_RETRIES || page > MAX_PAGES {
                info!(retries, page, "completed");
                break;
            }

            let query = self.engine.generate_query(&self.subdomains);

            info!(query, "searching");
            let resp = match self
                .engine
                .search(client.clone(), &query, page)
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(err = ?e, "failed to search");
                    retries += 1;
                    continue;
                }
            };

            let body = match resp.text().await {
                Ok(b) => b,
                Err(e) => {
                    warn!(err = ?e, "failed to parse search results");
                    retries += 1;
                    continue;
                }
            };

            // informs the executor that this task is about to block the thread
            // so any other tasks can be moved to a new worker thread
            tokio::task::block_in_place(|| {
                self.subdomains.extend(self.engine.extract(&body));
            });

            if found != self.subdomains.len() {
                found = self.subdomains.len();
                retries = max(0, retries - 1);
            } else {
                retries += 1;
            }
            page += 1;

            // Sleep after each page to avoid being blocked
            // tokio::time::sleep(Duration::from_secs(2)).await;
        }

        &self.subdomains
    }
}
