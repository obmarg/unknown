use std::collections::HashSet;

use tabled::{Table, Tabled};

use super::opts::ChangedOpts;
use crate::{
    git,
    workspace::{ProjectRef, Workspace, WorkspacePath},
};

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

impl super::opts::Format {
    pub fn actual_format(self) -> ActualFormat {
        match self {
            super::opts::Format::Auto if atty::is(atty::Stream::Stdout) => ActualFormat::Table,
            super::opts::Format::Auto => ActualFormat::Plain,
            super::opts::Format::Plain => ActualFormat::Plain,
            super::opts::Format::Table => ActualFormat::Table,
            super::opts::Format::Json => ActualFormat::Json,
            super::opts::Format::NdJson => ActualFormat::NdJson,
        }
    }
}
