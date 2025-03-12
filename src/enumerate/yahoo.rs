use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::{Client, Response, header};

use super::{Extract, SUBDOMAIN_RE_STR, Search};

// Yahoo seems to always return 7 results per page.
// Until we find a way to configure the number of results per page,
// don't change this value.
const PER_PAGE: usize = 7;
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
static YAHOO_RE: OnceLock<Regex> = OnceLock::new();

pub(crate) struct Yahoo {
    domain: String,
}

impl Yahoo {
    pub(crate) fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
        }
    }
}

impl Extract for Yahoo {
    fn extract(&self, input: &str) -> impl Iterator<Item = String> {
        let re = YAHOO_RE.get_or_init(|| {
            // Captures subdomains from Yahoo search result page, which is in HTML format.
            let domain = self.domain.replace(".", r"\.");
            let pat = format!(r#"<span>(?<subdomain>{SUBDOMAIN_RE_STR}\.{domain})<\/span>"#);

            // fail to compile regex is fatal since we can't not proceed without it
            Regex::new(&pat).expect("failed to compile regex")
        });

        re.captures_iter(input).map(|c| c["subdomain"].to_owned())
    }
}

impl Search for Yahoo {
    const NAME: &str = "Yahoo";
    const BASE_URL: &str = "https://search.yahoo.com/search";
    const MAX_PAGES: usize = 50;

    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        // TODO: consider limiting the number of subdomains to exclude
        let found = subdomains
            .iter()
            .fold(String::new(), |acc, d| format!("{} -domain:{}", acc, d));

        format!("site:{0} -domain:www.{0}{1}", self.domain, found)
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
            .get(Self::BASE_URL)
            .query(&[("p", query), ("b", b.to_string().as_ref())])
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
    #[case::empty(HashSet::new(), "site:example.com -domain:www.example.com")]
    #[case::single(
        HashSet::from(["app.example.com".to_owned()]), 
        "site:example.com -domain:www.example.com -domain:app.example.com"
    )]
    fn test_generate_query(#[case] subdomains: HashSet<String>, #[case] expected: &str) {
        let domain = "example.com";
        let google = Yahoo::new(domain);

        assert_eq!(google.generate_query(&subdomains), expected);
    }

    #[test]
    fn test_generate_query_multiple() {
        let domain = "example.com";
        let subdomains = HashSet::from([
            "first.example.com".to_owned(),
            "second.example.com".to_owned(),
        ]);
        let google = Yahoo::new(domain);
        let query = google.generate_query(&subdomains);

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
        r#"<span>app.example.com</span>"#,
        vec!["app.example.com"]
    )]
    #[case::with_hyphens(
        r#"<span>with-hypen.example.com</span>"#,
        vec!["with-hypen.example.com"]
    )]
    #[case::multi_level(
        r#"<span>level1.level2.example.com</span>"#,
        vec!["level1.level2.example.com"]
    )]
    #[case::multi_matches(
        r#"
        <span>first.example.com</span>
        <span>second.example.com</span>
        <span>fourth.third.example.com</span>
        "#,
        vec!["first.example.com", "second.example.com", "fourth.third.example.com"]
    )]
    fn test_extract_single_level(#[case] input: &str, #[case] expected: Vec<&str>) {
        let yahoo = Yahoo::new("example.com");
        let results: Vec<String> = yahoo.extract(input).collect();
        assert_eq!(expected, results);
    }
}
