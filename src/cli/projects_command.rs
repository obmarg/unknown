use tabled::{Table, Tabled};

use crate::workspace::{Workspace, WorkspacePath};

#[derive(clap::Parser)]
pub struct ProjectsOpts {
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

pub fn run(workspace: Workspace, opts: ProjectsOpts) -> miette::Result<()> {
    let outputs = workspace.projects().map(|p| Output {
        name: p.name.clone(),
        path: p.root.clone(),
    });

    match opts.format.actual_format() {
        ActualFormat::Plain => {
            for project in outputs {
                println!("{}", project.name)
            }
        }
        ActualFormat::Table => {
            println!("{}", Table::new(outputs));
        }
        ActualFormat::Json => {
            let outputs = outputs.collect::<Vec<_>>();
            print!("{}", serde_json::to_string(&outputs).unwrap())
        }
        ActualFormat::NdJson => {
            for output in outputs {
                println!("{}", serde_json::to_string(&output).unwrap())
            }
        }
    }

    Ok(())
}

#[derive(serde::Serialize, Tabled)]
pub struct Output {
    name: String,
    path: WorkspacePath,
}

#[derive(Clone, Copy, Debug)]
pub enum ActualFormat {
    Plain,
    Table,
    Json,
    NdJson,
}

impl Format {
    pub fn actual_format(self) -> ActualFormat {
        match self {
            Format::Auto if atty::is(atty::Stream::Stdout) => ActualFormat::Table,
            Format::Auto => ActualFormat::Plain,
            Format::Plain => ActualFormat::Plain,
            Format::Table => ActualFormat::Table,
            Format::Json => ActualFormat::Json,
            Format::NdJson => ActualFormat::NdJson,
        }
    }
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

impl std::str::FromStr for Format {
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
