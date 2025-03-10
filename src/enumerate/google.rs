use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::header::{self};
use reqwest::{Client, Response};

use super::{Extract, Search};

// Google requires JavaScript to be enabled for `/search` public endpoint.
// Pure-text browsers seem to be exempt from this requirement.
// User-Agent values of these browsers can be used to get raw HTML search results.
// More info: https://news.ycombinator.com/item?id=42747092
//
// Values that appear to work:
// - "Lynx/2.8.6rel.5 libwww-FM/2.14"
// - "w3m/0.5.3"
const USER_AGENT: &str = "Lynx/2.8.6rel.5 libwww-FM/2.14";
static RE: OnceLock<Regex> = OnceLock::new();

pub(crate) struct Google {
    domain: String,
}

impl Google {
    pub(crate) fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
        }
    }
}

impl Extract for Google {
    async fn extract<T>(&self, input: T) -> HashSet<String>
    where
        T: AsRef<String> + Send + 'static,
    {
        let re = RE.get_or_init(|| {
            // Captures subdomains from Google search result page, which is in HTML format.
            // The pattern matches valid domain names followed by the HTML entity &#8250; (›)
            // It ensures proper domain name format:
            //  1. One or more labels separated by dots
            //  2. Each label can contain alphanumeric characters and hyphens
            //  3. Hyphens can appear in the middle, but not at start/end
            //  4. No empty labels (consecutive dots)
            let domain = self.domain.replace(".", r"\.");
            let pat = format!(
                r#"(?<subdomain>([[:alnum:]](?:[[:alnum:]-]*[[:alnum:]])?)(?:\.[[:alnum:]](?:[[:alnum:]-]*[[:alnum:]])?)*\.{domain}) &#8250;"#,
            );
            // fail to compile regex is fatal since we can't not proceed without it
            Regex::new(&pat).expect("failed to compile regex")
        });

        // TODO: handle errors
        tokio::task::spawn_blocking(move || {
            re.captures_iter(input.as_ref())
                .map(|c| c["subdomain"].to_owned())
                .collect::<HashSet<_>>()
        })
        .await
        .unwrap()
    }
}

impl Search for Google {
    const BASE_URL: &str = "https://www.google.com/search";

    /// Constructs a search query for subdomain enumeration
    ///
    /// Creates a query using Google's search syntax. The query structure is:
    /// * `site:domain.com` to restrict results to the target domain
    /// * `-www.domain.com` to exclude the default `www` subdomain
    /// * Additional `-subdomain.domain.com` terms for each known subdomain
    ///
    /// For example:
    ///
    ///  - If no subdomains have been discovered yet, the query will be:
    /// `site:example.com -www.example.com`
    ///
    ///  - If subdomains have been discovered, the query will be:
    /// `site:example.com -www.example.com -subdomain1.example.com -subdomain2.example.com`
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        // TODO: limit the number of subdomains to exclude to 10
        let found = subdomains
            .iter()
            .fold(String::new(), |acc, d| format!("{} -{}", acc, d));

        format!("site:{0} -www.{0}{1}", self.domain, found)
    }

    async fn search(
        &self,
        client: Client,
        query: &str,
        page: usize,
    ) -> Result<Response, reqwest::Error> {
        // Google's default number of results per page is 10.
        // We offset the start by 10 for each page.
        let start = page * 10;
        client
            .get(Self::BASE_URL)
            .query(&[
                ("q", query),
                ("hl", "en-US"),
                ("start", start.to_string().as_ref()), // result offset from num, 0 = first page, 25 = second page, etc.
                ("filter", "0"), // duplicates content filter, 0 = include duplicates
            ])
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::empty(vec![], "site:example.com -www.example.com")]
    #[case::single(vec!["app.example.com"], "site:example.com -www.example.com -app.example.com")]
    // #[case::multiple(vec!["app.example.com", "api.example.com"], "site:example.com -www.example.com -app.example.com -api.example.com")]
    fn test_generate_query(#[case] subdomains: Vec<&'static str>, #[case] expected: &str) {
        let domain = "example.com";
        let mut google = Google::new(domain);

        let subdomains = subdomains.into_iter().map(|s| s.to_owned()).collect();

        assert_eq!(google.generate_query(&subdomains), expected);
    }
}
