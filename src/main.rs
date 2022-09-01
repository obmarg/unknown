use std::path::{Path, PathBuf};

use crate::config::ParsingError;

mod config;
mod workspace;

// TODO: Consider using camino::Utf8PathBuf everywhere instead...

fn main() {
    // So, suboptimal startup approach:
    // Walk the parent dir tree till we find a project.kdl
    // - If this is a project, keep walking till we find
    //   a workspace.
    // Read that workspace file.
    // Use the globs within to scan for any project files.
    // Read all the project files.
    //
    // Although ideally we only want to do this if a command
    // that requires this data has been called.
    // To be fair, that'll be most of them.
    //
    // May need a way to be smarter for speed purposes.
    // If projects were referred to by their paths that could
    // speed things up significantly.
    // But depends how slow it actually is to read all these project
    // files.  Probably an over optimisation initially.

    let workspace_path = find_workspace_file().expect("couldn't find workspace file");
    let workspace_file = config::workspace_from_str(
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )
    .map_err(ParsingError::into_report)
    .expect("Failed to parse workspace data");

    let project_config_paths = workspace_file
        .project_paths
        .iter()
        .flat_map(|project_path| find_project_files(workspace_path.parent().unwrap(), project_path))
        .collect::<Vec<_>>();

    let project_files = project_config_paths
        .iter()
        .map(|f| {
            let path = f
                .canonicalize()
                .expect("project path to be canonicalizable");

            config::parse_project_file(
                &path,
                &std::fs::read_to_string(f).expect("couldn't read project file"),
            )
            .map_err(ParsingError::into_report)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let workspace = workspace::Workspace::new(workspace_file, project_files);

    println!("{workspace:?}");
}

fn find_project_files(root: &Path, project_path: &str) -> Vec<PathBuf> {
    let glob = match project_path.ends_with('/') {
        true => format!("{project_path}project.kdl"),
        false => format!("{project_path}/project.kdl"),
    };
    // TODO: At some point want to update this to use ignore.  not now though
    glob::glob(&glob)
        .expect("Project path patterns broke innit")
        .map(|r| r.expect("something wrong with the result"))
        .collect()
}

fn find_workspace_file() -> Option<PathBuf> {
    let mut current_path = std::env::current_dir().ok()?;

    while current_path.parent().is_some() {
        current_path.push("workspace.kdl");
        if current_path.exists() {
            return Some(current_path);
        }
        current_path.pop();
        current_path.pop();
    }

    None
}
