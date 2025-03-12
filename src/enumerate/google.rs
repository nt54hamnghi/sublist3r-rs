use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::header::{self};
use reqwest::{Client, Response};

use super::{Extract, SUBDOMAIN_RE_STR, Search};

// Google requires JavaScript to be enabled for `/search` public endpoint.
// Pure-text browsers seem to be exempt from this requirement.
// User-Agent values of these browsers can be used to get raw HTML search results.
// More info: https://news.ycombinator.com/item?id=42747092
//
// Values that appear to work:
// - "Lynx/2.8.6rel.5 libwww-FM/2.14"
// - "w3m/0.5.3"
const USER_AGENT: &str = "Lynx/2.8.6rel.5 libwww-FM/2.14";
const PER_PAGE: usize = 20;
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
    fn extract(&self, input: &str) -> impl Iterator<Item = String> {
        let re = RE.get_or_init(|| {
            // Captures subdomains from Google search result page, which is in HTML format.
            // The pattern matches valid domain names followed by the HTML entity &#8250; (›)
            let domain = self.domain.replace(".", r"\.");
            let pat = format!(r#"(?<subdomain>{SUBDOMAIN_RE_STR}\.{domain}) &#8250;"#);

            // fail to compile regex is fatal since we can't not proceed without it
            Regex::new(&pat).expect("failed to compile regex")
        });

        re.captures_iter(input).map(|c| c["subdomain"].to_owned())
    }
}

impl Search for Google {
    const NAME: &str = "Google";
    const BASE_URL: &str = "https://www.google.com/search";
    const MAX_PAGES: usize = 20;

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
    ///    `site:example.com -www.example.com`
    ///
    ///  - If subdomains have been discovered, the query will be:
    ///    `site:example.com -www.example.com -subdomain1.example.com -subdomain2.example.com`
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        // TODO: consider limiting the number of subdomains to exclude
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
        // Google's search pagination uses a 0-based index.
        // For `PER_PAGE` = 20, the pagination is as follows:
        // `page` = 0 (1st page): start=0  (results 1-20)
        // `page` = 1 (2nd page): start=20 (results 21-40)
        // `page` = 2 (3rd page): start=40 (results 41-60)
        // and so on...
        let start = page * PER_PAGE;

        client
            .get(Self::BASE_URL)
            .query(&[
                ("q", query),
                ("hl", "en-US"),
                ("num", PER_PAGE.to_string().as_ref()), // number of search results per page
                ("start", start.to_string().as_ref()),  // starting position for pagination
                ("filter", "0"), // duplicates content filter, 0 = include duplicates
            ])
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty(HashSet::new(), "site:example.com -www.example.com")]
    #[case::single(
        HashSet::from(["app.example.com".to_owned()]), 
        "site:example.com -www.example.com -app.example.com"
    )]
    fn test_generate_query(#[case] subdomains: HashSet<String>, #[case] expected: &str) {
        let domain = "example.com";
        let google = Google::new(domain);

        assert_eq!(google.generate_query(&subdomains), expected);
    }

    #[test]
    fn test_generate_query_multiple() {
        let domain = "example.com";
        let subdomains = HashSet::from([
            "first.example.com".to_owned(),
            "second.example.com".to_owned(),
        ]);
        let google = Google::new(domain);
        let query = google.generate_query(&subdomains);

        let (expected1, expected2) = (
            "site:example.com -www.example.com -first.example.com -second.example.com",
            "site:example.com -www.example.com -second.example.com -first.example.com",
        );

        assert!(query == expected1 || query == expected2);
    }

    #[rstest]
    #[case::empty("", vec![])]
    #[case::no_matches("no matches found", vec![])]
    #[case::basic(
        r#"<div>app.example.com &#8250; Text</div>"#,
        vec!["app.example.com"]
    )]
    #[case::with_hyphens(
        r#"<div>with-hypen.example.com &#8250; Text</div>"#,
        vec!["with-hypen.example.com"]
    )]
    #[case::multi_level(
        r#"<div>level1.level2.example.com &#8250; Text</div>"#,
        vec!["level1.level2.example.com"]
    )]
    #[case::multi_matches(
        r#"
        <div>first.example.com &#8250; Text</div>
        <div>second.example.com &#8250; Text</div>
        <div>fourth.third.example.com &#8250; Text</div>
        "#,
        vec!["first.example.com", "second.example.com", "fourth.third.example.com"]
    )]
    fn test_extract_single_level(#[case] input: &str, #[case] expected: Vec<&str>) {
        let google = Google::new("example.com");
        let results: Vec<String> = google.extract(input).collect();
        assert_eq!(expected, results);
    }
}
