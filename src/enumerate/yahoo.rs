use std::borrow::Cow;
use std::collections::HashSet;

use reqwest::{Client, Response, header};

use super::{DEFAULT_USER_AGENT, Extract, SUBDOMAIN_RE_STR, Search, Settings};

// Yahoo seems to always return 7 results per page.
// Until we find a way to configure the number of results per page,
// don't change this value.
const PER_PAGE: usize = 7;
const SETTINGS: Settings = Settings {
    name: "Yahoo",
    base_url: "https://search.yahoo.com/search",
    user_agent: DEFAULT_USER_AGENT,
    max_rounds: 50,
};

#[derive(Extract)]
#[extract(pattern = r#"<span>(?<subdomain>{SUBDOMAIN_RE_STR}\.{domain})<\/span>"#)]
pub struct Yahoo {
    #[extract(domain)]
    domain: String,
}

impl Yahoo {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

impl Search for Yahoo {
    fn next_query(&self, subdomains: &HashSet<String>) -> Option<Cow<'_, str>> {
        let found = subdomains
            .iter()
            .take(15)
            .fold(String::new(), |acc, d| format!("{acc} -domain:{d}"));

        let query = format!("site:{0} -domain:www.{0}{1}", self.domain, found);
        Some(Cow::Owned(query))
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
        // Yahoo's search results use a 1-based index.
        // For `PER_PAGE` = 7, the pagination is as follows:
        // `page` = 0 (1st page): results 1-7
        // `page` = 1 (2nd page): results 8-14
        // `page` = 2 (3rd page): results 15-21, etc.
        let b = (page * PER_PAGE) + 1;

        client
            .get(SETTINGS.base_url)
            .query(&[("p", query)])
            .query(&[("b", b)])
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
    #[case::empty(HashSet::new(), "site:example.com -domain:www.example.com")]
    #[case::single(
        HashSet::from(["app.example.com".to_owned()]), 
        "site:example.com -domain:www.example.com -domain:app.example.com"
    )]
    fn test_generate_query(#[case] subdomains: HashSet<String>, #[case] expected: &str) {
        let domain = "example.com";
        let google = Yahoo::new(domain);
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
        let google = Yahoo::new(domain);
        let query = google.next_query(&subdomains).unwrap();

        let (expected1, expected2) = (
            "site:example.com -domain:www.example.com -domain:first.example.com -domain:second.example.com",
            "site:example.com -domain:www.example.com -domain:second.example.com -domain:first.example.com",
        );

        assert!(query == expected1 || query == expected2);
    }

    #[rstest]
    #[case::empty("", vec![])]
    #[case::no_matches("no matches found", vec![])]
    #[case::basic(
        r"<span>app.example.com</span>",
        vec!["app.example.com"]
    )]
    #[case::with_hyphens(
        r"<span>with-hypen.example.com</span>",
        vec!["with-hypen.example.com"]
    )]
    #[case::multi_level(
        r"<span>level1.level2.example.com</span>",
        vec!["level1.level2.example.com"]
    )]
    #[case::multi_matches(
        r"
        <span>first.example.com</span>
        <span>second.example.com</span>
        <span>fourth.third.example.com</span>
        ",
        vec!["first.example.com", "second.example.com", "fourth.third.example.com"]
    )]
    fn test_extract(#[case] input: &str, #[case] expected: Vec<&str>) {
        let mut yahoo = Yahoo::new("example.com");
        let results = yahoo.extract(input);

        let expected: HashSet<String> = expected.into_iter().map(String::from).collect();
        assert_eq!(expected, results);
    }
}
