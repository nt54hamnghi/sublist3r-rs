use std::collections::HashSet;

use reqwest::{Client, Response, header};

use super::{Extract, Search, Settings};

const PER_PAGE: usize = 10;
const SETTINGS: Settings = Settings {
    name: "Bing",
    base_url: "https://www.bing.com/search",
    // https://learn.microsoft.com/en-us/bing/search-apis/bing-web-search/reference/headers
    user_agent: "Mozilla/5.0 (Windows NT 6.3; WOW64; Trident/7.0; Touch; rv:11.0) like Gecko",
    max_pages: 30,
};
/// A random client id that Bing uses to identify the client to ensure consistent results
const X_MSEDGE_CLIENT_ID: &str = "sublist3r-rs-bing";

#[derive(Extract)]
#[extract(pattern = r#"<cite>https:\/\/(?<subdomain>.*?\.{domain}).*?<\/cite>"#)]
pub(crate) struct Bing {
    #[extract(domain)]
    domain: String,
}

impl Bing {
    pub(crate) fn new(domain: impl Into<String>) -> Self {
        // TODO: validate domain
        Self {
            domain: domain.into(),
        }
    }
}

impl Search for Bing {
    fn generate_query(&self, subdomains: &HashSet<String>) -> String {
        // TODO: consider limiting the number of subdomains to exclude
        let found = subdomains
            .iter()
            .fold(String::new(), |acc, d| format!("{} -{}", acc, d));

        format!("domain:{0} -www.{0}{1}", self.domain, found)
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
        let offset = page * PER_PAGE;

        client
            .get(SETTINGS.base_url)
            .query(&[("q", query)])
            .query(&[("count", PER_PAGE)])
            .query(&[("offset", offset)])
            .header(header::USER_AGENT, SETTINGS.user_agent)
            .header(header::ACCEPT, "text/html")
            .header("X-MSEdge-ClientID", X_MSEDGE_CLIENT_ID)
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty("", vec![])]
    #[case::no_matches("no matches found", vec![])]
    #[case::basic(
        r#"<cite>https://app.example.com</cite>"#,
        vec!["app.example.com"]
    )]
    #[case::basic_with_extra_text(
        r#"<cite>https://app.example.com</cite> - Bing"#,
        vec!["app.example.com"]
    )]
    #[case::with_hyphens(
        r#"<cite>https://with-hypen.example.com</cite>"#,
        vec!["with-hypen.example.com"]
    )]
    #[case::multi_level(
        r#"<cite>https://level1.level2.example.com</cite>"#,
        vec!["level1.level2.example.com"]
    )]
    #[case::multi_matches(
        r#"
        <cite>https://first.example.com</cite>
        <cite>https://second.example.com</cite>
        <cite>https://fourth.third.example.com</cite>
        "#,
        vec!["first.example.com", "second.example.com", "fourth.third.example.com"]
    )]
    fn test_extract(#[case] input: &str, #[case] expected: Vec<&str>) {
        let bing = Bing::new("example.com");
        let results = bing.extract(input);

        let expected: HashSet<String> = expected.into_iter().map(String::from).collect();
        assert_eq!(expected, results);
    }
}
