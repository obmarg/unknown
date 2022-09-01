use std::str::FromStr;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Reports which projects have changed in the repository, including dependants that may be affected
    Changed(ChangedOpts),
}

#[derive(Parser)]
pub struct ChangedOpts {
    /// A git ref to compare against.
    #[clap(long, default_value_t = String::from("HEAD"), value_parser)]
    pub since: String,

    /// The format to output.
    ///
    /// Can be one of auto, plain, table, json, ndjson.
    ///
    /// Defaults to showing a table if running interactively, plain otherwise.
    #[clap(long, default_value_t = Format::Auto)]
    pub format: Format,
}

pub enum Format {
    Auto,
    Plain,
    Table,
    Json,
    NdJson,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Auto => write!(f, "auto"),
            Format::Plain => write!(f, "plain"),
            Format::Table => write!(f, "table"),
            Format::Json => write!(f, "json"),
            Format::NdJson => write!(f, "ndjson"),
        }
    }
}

impl FromStr for Format {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "auto" => Format::Auto,
            "plain" => Format::Plain,
            "table" => Format::Table,
            "json" => Format::Json,
            "ndjson" => Format::NdJson,
            _ => miette::bail!("Unknown format: {s}.  Expected one of auto, plain, json, ndjson"),
        })
    }
}
