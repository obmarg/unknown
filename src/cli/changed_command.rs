use std::collections::HashSet;

use camino::Utf8Path;
use tabled::{Table, Tabled};

use crate::{
    config::ValidPath,
    git,
    workspace::{ProjectInfo, Workspace},
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

#[derive(Clone)]
pub enum Format {
    Auto,
    Plain,
    Table,
    Json,
    NdJson,
}

pub fn run(workspace: Workspace, opts: ChangedOpts) -> miette::Result<()> {
    let files_changed = git::files_changed(git::Mode::Feature(opts.since))?;

    let repo_root = git::repo_root().expect("need to find repo root");
    let repo_root = repo_root.as_path();

    if workspace.root_path() != repo_root {
        return Err(miette::miette!("The workspace must be at the repo root currently (mostly because I'm lazy though, PRs welcome)"));
    }

    // Note: Possibly some room for optimisation here where we go project-by-project
    // instead of file-by-file.  Could let us short-circuit things a bit, rather than
    // needing to compare every file against every project

    let projects_changed = files_changed
        .into_iter()
        .flat_map(|file| {
            workspace
                .projects()
                .filter(|project| project.contains(&file))
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();

    let graph = workspace.graph();

    let projects_affected = projects_changed
        .into_iter()
        .flat_map(|p| graph.walk_project_dependents(p.project_ref()))
        .collect::<HashSet<_>>();

    let mut outputs = projects_affected
        .into_iter()
        .map(|project_ref| {
            let project = project_ref.lookup(&workspace);
            Output {
                name: project.name.clone(),
                path: project.root.clone(),
            }
        })
        .collect::<Vec<_>>();
    outputs.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));

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
    path: ValidPath,
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

impl ProjectInfo {
    fn contains(&self, path: &Utf8Path) -> bool {
        path.starts_with(self.root.as_subpath())
            && (self.path_exclusions.is_empty()
                || !self
                    .path_exclusions
                    .iter()
                    .any(|exclusion| path.starts_with(exclusion.as_subpath())))
    }
}
