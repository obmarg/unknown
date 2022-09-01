use std::collections::HashSet;

use tabled::{Table, Tabled};

use crate::{
    git,
    workspace::{ProjectRef, Workspace, WorkspacePath},
};

#[derive(clap::Parser)]
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

pub fn run(workspace: Workspace, opts: ChangedOpts) -> miette::Result<()> {
    let files_changed = git::files_changed(git::Mode::Feature(opts.since)).unwrap();

    let repo_root = git::repo_root().expect("need to find repo root");
    let repo_root = repo_root.as_path();

    let projects_changed = files_changed
        .into_iter()
        .map(|p| repo_root.join(p))
        .flat_map(|file| {
            workspace
                .projects()
                .filter(|project| file.starts_with(&project.root))
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();

    // Ok, so basic monobuild mode requires:
    // - Map projects_changed into a set of changed & dependant projects.
    let projects_affected = projects_changed
        .into_iter()
        .flat_map(|p| {
            workspace
                .graph
                .walk_project_dependencies(ProjectRef::new(&p.name))
        })
        .collect::<HashSet<_>>();

    // TODO: Probably topsort the output.
    let outputs = projects_affected.into_iter().map(|project_ref| {
        let project = workspace.lookup(&project_ref);
        // TODO: Probably want these paths to be relative to the repo root.
        //
        // Do I want my own abstraction that covers this?
        // Maybe.
        Output {
            name: project.name.clone(),
            path: project.root.clone(),
        }
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
    // TODO: Might be good to include the reason it was included in here as well.
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