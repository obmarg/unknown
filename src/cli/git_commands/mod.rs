use std::collections::HashSet;

use clap::Parser;

use crate::{config::load_project_files, git, workspace::Workspace};

#[derive(Parser)]
pub enum GitCommand {
    /// Adds a project and its dependencies to the current sparse-checkout
    Checkout(CheckoutOptions),
}

#[derive(Parser)]
pub struct CheckoutOptions {
    /// The path to one or more projects to add to the sparse-checkout
    project_paths: Vec<String>,
}

pub fn run(workspace: Workspace, command: GitCommand) -> miette::Result<()> {
    match command {
        GitCommand::Checkout(opts) => run_checkout(workspace, opts),
    }
}

fn run_checkout(workspace: Workspace, opts: CheckoutOptions) -> miette::Result<()> {
    let root_path = workspace.root_path();
    let mut paths = opts
        .project_paths
        .into_iter()
        .map(|p| root_path.subpath(p))
        .collect::<Result<Vec<_>, _>>()?;

    let mut loaded_paths = HashSet::new();

    git::sparse_checkout_init()?;

    while let Some(path) = paths.pop() {
        loaded_paths.insert(path.clone());
        // TODO: We may to make sure the CWD is the root of the repo to do this?
        // Or otherwise figure out how to make it relative to the workspace root...
        git::sparse_checkout_add(path.subpath())?;
        let project_files = load_project_files(root_path, &path, &workspace.info.project_paths)?;
        for project in project_files {
            for dep_path in project.unvalidated_dependency_paths() {
                let relative_path = path.join(&dep_path.clone().into_raw().expect("a raw path"))?;
                if !loaded_paths.contains(&relative_path) {
                    paths.push(relative_path);
                }
            }
        }
    }

    Ok(())
}

// So, things I need to do:
// 1. Change config to only accept dep specs in path form.
//    - Paths can be relatvive or absolute to the root of the repo.
//      - Paths should (probably) be normalised to absolute before being passed to
//         workspace.
//      - Optionally should be able to make sure paths exist before finishing parsing?
//        So we can get proper spanned errors on missing paths.
// 2. Make sure the above works with lookups etc.
//    Do we want this integrated into ProjectRef? or have it as a separate concept?
//    Not sure.
// 3. Implement sparse checkout like:
//    while let path in paths { git sparse_checkout add; path.push(deps at path); }
