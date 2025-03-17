use std::str::FromStr;

use clap::{Parser, ValueEnum};
use url::{Host, Url};

use crate::enumerate::EngineChoice;

#[derive(Parser, Debug)]
#[command(name = "s7r")]
#[command(author, version, about, long_about)]
#[command(arg_required_else_help = true)]
#[command(verbatim_doc_comment, propagate_version = true)]
pub struct Cli {
    /// Domain name to enumerate it's subdomains
    #[arg(short, long)]
    pub domain: Domain,

    /// Specify a comma-separated list of search engines
    #[arg(short, long, value_delimiter = ',')]
    pub engines: Vec<EngineChoice>,
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
