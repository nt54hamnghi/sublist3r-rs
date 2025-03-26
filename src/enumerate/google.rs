use std::borrow::Cow;
use std::collections::HashSet;

use reqwest::header::{self};
use reqwest::{Client, Response};

use super::{Extract, Search, Settings};

const PER_PAGE: usize = 20;
const SETTINGS: Settings = Settings {
    name: "Google",
    base_url: "https://www.google.com/search",
    // Google requires JavaScript to be enabled for `/search` public endpoint.
    // Pure-text browsers seem to be exempt from this requirement.
    // User-Agent values of these browsers can be used to get raw HTML search results.
    // More info: https://news.ycombinator.com/item?id=42747092
    //
    // Values that appear to work:
    // - "Lynx/2.8.6rel.5 libwww-FM/2.14"
    // - "w3m/0.5.3"
    user_agent: "Lynx/2.8.6rel.5 libwww-FM/2.14",
    max_rounds: 20,
};

#[derive(Extract)]
#[extract(pattern = r#"<span.*?>(?<subdomain>[[:alnum:]\-\.]*?\.{domain})\s&#8250;.*?<\/span>"#)]
pub struct Google {
    #[extract(domain)]
    domain: String,
}

impl Google {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

impl Search for Google {
    fn settings(&self) -> Settings {
        SETTINGS
    }

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
    fn next_query(&self, subdomains: &HashSet<String>) -> Option<Cow<'_, str>> {
        // TODO: consider limiting the number of subdomains to exclude
        let found = subdomains
            .iter()
            .fold(String::new(), |acc, d| format!("{acc} -{d}"));

        let query = format!("site:{0} -www.{0}{1}", self.domain, found);
        Some(Cow::Owned(query))
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
            .get(SETTINGS.base_url)
            .query(&[("q", query)])
            .query(&[("hl", "en-US")])
            .query(&[("num", PER_PAGE)]) // number of search results per page
            .query(&[("start", start)]) // starting position for pagination
            .query(&[("filter", "0")]) // duplicates content filter, 0 = include duplicates
            .header(header::USER_AGENT, SETTINGS.user_agent)
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
        let query = google.next_query(&subdomains).unwrap();

        assert_eq!(query, expected);
    }

    #[test]
    fn test_generate_query_multiple() {
        let domain = "example.com";
        let subdomains = HashSet::from([
            "first.example.com".to_owned(),
            "second.example.com".to_owned(),
        ]);
        let google = Google::new(domain);
        let query = google.next_query(&subdomains).unwrap();

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
        r"<span>app.example.com &#8250; Text</span>",
        vec!["app.example.com"]
    )]
    #[case::with_hyphens(
        r#"<span class="cite">with-hypen.example.com &#8250; Text</span>"#,
        vec!["with-hypen.example.com"]
    )]
    #[case::multi_level(
        r"<span>level1.level2.example.com &#8250; Text</span>",
        vec!["level1.level2.example.com"]
    )]
    #[case::multi_matches(
        r"
        <span>first.example.com &#8250; Text</span>
        <span>second.example.com &#8250; Text</span>
        <span>fourth.third.example.com &#8250; Text</span>
        ",
        vec!["first.example.com", "second.example.com", "fourth.third.example.com"]
    )]
    fn test_extract(#[case] input: &str, #[case] expected: Vec<&str>) {
        let mut google = Google::new("example.com");
        let results = google.extract(input);

        let expected: HashSet<String> = expected.into_iter().map(String::from).collect();
        assert_eq!(expected, results);
    }
}
