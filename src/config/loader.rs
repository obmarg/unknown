use std::path::{Path, PathBuf};

use super::{
    parse_project_file, parse_task_file, parse_workspace_file, ParsingError, ProjectFile,
    WorkspaceFile,
};

pub fn load_config_from_cwd() -> Result<(WorkspaceFile, Vec<ProjectFile>), miette::Report> {
    let workspace_path = find_workspace_file().ok_or(MissingWorkspaceFile)?;
    let workspace_file = parse_workspace_file(
        &workspace_path,
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )
    .map_err(ParsingError::into_report)?;

    let project_config_paths = workspace_file
        .config
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

            parse_project_file(
                &path,
                &std::fs::read_to_string(f).expect("couldn't read project file"),
            )
            .map_err(ParsingError::into_report)
            .and_then(|project_file| import_tasks(project_file, &workspace_path))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((workspace_file, project_files))
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("Couldn't find a workspace.kdl")]
#[diagnostic(help("nabs requires a workspace.kdl in the current directory or a parent directory"))]
struct MissingWorkspaceFile;

fn find_project_files(root: &Path, project_path: &str) -> Vec<PathBuf> {
    let root = root.as_os_str().to_str().unwrap();
    let project_path = match project_path.starts_with('/') {
        true => format!("{root}{project_path}"),
        false => format!("{root}/{project_path}"),
    };
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
            return Some(
                current_path
                    .canonicalize()
                    .expect("to be able to canonicalize root path"),
            );
        }
        current_path.pop();
        current_path.pop();
    }

    None
}

fn import_tasks(
    mut project_file: ProjectFile,
    workspace_root: &Path,
) -> Result<ProjectFile, miette::Report> {
    let mut imports = std::mem::take(&mut project_file.config.tasks.imports);
    while let Some(import) = imports.pop() {
        let path = match import.starts_with('/') {
            true => workspace_root.join(import),
            false => project_file.project_root.join(import),
        };
        let parsed = parse_task_file(
            &path,
            // TODO: This one definitely shouldn't be an expect, need an actual error.
            &std::fs::read_to_string(&path).expect("couldn't read task file"),
        )
        .map_err(ParsingError::into_report)?;

        imports.extend(parsed.config.imports);

        project_file.config.tasks.tasks.extend(parsed.config.tasks);
    }

    Ok(project_file)
}
