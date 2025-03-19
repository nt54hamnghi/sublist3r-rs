use std::str::FromStr;

use clap::{Command, Parser};
use clap_complete::{Generator, Shell, generate};
use owo_colors::OwoColorize;
use url::{Host, Url};

use crate::enumerate::EngineChoice;

pub const BANNER: &str = r#"
            _____    
       ____/__  /____
      / ___/ / / ___/
     (__  ) / / /    
    /____/ /_/_/     
    
    @nt54hamnghi
"#;

pub const WARNINGS: &str = r#"
WARNING:
[!] This tool is for educational purposes only.
[!] Users are responsible for their actions.
[!] Please respect the terms of use of all data sources used by this tool.
"#;

pub fn header() -> String {
    format!("{}\n{}", BANNER.purple(), WARNINGS.yellow())
}

/// A Rust rewrite of Sublist3r
#[derive(Parser, Debug)]
#[command(name = "s7r")]
#[command(author, version, about, long_about)]
#[command(before_help = header(), before_long_help = header())]
#[command(arg_required_else_help = true)]
#[command(verbatim_doc_comment, propagate_version = true)]
pub struct Cli {
    /// Domain name to enumerate it's subdomains
    #[arg(short, long, required_unless_present = "completion")]
    pub domain: Option<Domain>,

    /// Specify a comma-separated list of search engines
    #[arg(short, long, value_delimiter = ',')]
    pub engines: Vec<EngineChoice>,

    /// Enable Verbosity and display results in realtime
    #[arg(short, long)]
    pub verbose: bool,

    /// Generate completion for the given shell
    #[arg(short, long, conflicts_with_all = ["domain", "engines", "verbose"])]
    pub completion: Option<Shell>,
}

#[derive(Debug, Clone)]
pub enum Domain {
    Url(Url),
    Host(Host),
}

impl FromStr for Domain {
    type Err = url::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Url::parse(s)
            .map(Domain::Url)
            .or_else(|_| Host::parse(s).map(Domain::Host))
    }
}

impl Domain {
    pub fn domain(&self) -> Option<&str> {
        match self {
            Domain::Url(u) => u.domain(),
            Domain::Host(h) => match h {
                Host::Domain(d) => Some(d),
                _ => None,
            },
        }
    }
}

pub fn print_completions<G: Generator>(g: G, c: &mut Command) {
    generate(g, c, c.get_name().to_string(), &mut std::io::stdout());
}
