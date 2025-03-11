use std::cmp::max;
use std::collections::HashSet;

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use reqwest::{Client, Response};

pub(crate) mod google;

const MAX_PAGES: usize = 20;

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

// temporary alias for `anyhow::Error`
// might need to replace it with a custom error type defined with thiserror
type Error = anyhow::Error;

impl<E> Enumerator<E>
where
    E: Search + Extract,
{
    pub async fn enumerate(&mut self, client: Client) -> Result<&HashSet<String>, Error> {
        let mut page = 0;
        let mut retries = 0;
        let mut current = 0;

        loop {
            if retries > 5 || page > MAX_PAGES {
                break;
            }

            let query = self.engine.generate_query(&self.subdomains);

            let Ok(resp) = self
                .engine
                .search(client.clone(), &query, page)
                .await
                .and_then(|r| r.error_for_status())
            else {
                retries += 1;
                continue;
            };

            let Ok(body) = resp.text().await else {
                retries += 1;
                continue;
            };

            // informs the executor that this task is about to block the thread
            // so any other tasks can be moved to a new worker thread
            tokio::task::block_in_place(|| {
                self.subdomains.extend(self.engine.extract(&body));
            });

            if current != self.subdomains.len() {
                current = self.subdomains.len();
                retries = max(0, retries - 1);
            } else {
                retries += 1;
            }
            page += 1;
            // tokio::time::sleep(Duration::from_secs(2)).await;
        }

        Ok(&self.subdomains)
    }
}
