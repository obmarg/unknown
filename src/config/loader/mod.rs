use std::path::{self, Path, PathBuf};

use miette::Context;

use super::{
    parse_project_file, parse_task_file, parse_workspace_file, ParsingError, ProjectFile,
    WorkspaceFile,
};

#[cfg(test)]
mod tests;

pub fn load_config_from_path(
    current_path: PathBuf,
) -> Result<(WorkspaceFile, Vec<ProjectFile>), miette::Report> {
    let current_path = current_path
        .canonicalize()
        .expect("to be able to canonicalise current path");

    let workspace_path = find_workspace_file(current_path).ok_or(MissingWorkspaceFile)?;
    let workspace_file = parse_workspace_file(
        &workspace_path,
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )?;

    let workspace_root = workspace_path.parent().unwrap();

    let project_config_paths = workspace_file
        .config
        .project_paths
        .iter()
        .flat_map(|project_path| find_project_files(workspace_root, project_path))
        .collect::<Vec<_>>();

    let project_files = project_config_paths
        .iter()
        .map(|path| {
            let canonical_path = path
                .canonicalize()
                .expect("project path to be canonicalizable");

            let Ok(relative_path) = canonical_path.strip_prefix(workspace_root) else {
                return Err(ProjectImportError::FileNotInWorkspace { file_path: canonical_path, workspace_root: workspace_root.to_owned() })
            };

            let project_file = parse_project_file(
                relative_path,
                &std::fs::read_to_string(&canonical_path).expect("couldn't read project file"),
            ).map_err(|e| ProjectImportError::ParsingError(e, relative_path.to_owned()))?;

            Ok(import_tasks(project_file, workspace_root)?)
        })
        .collect::<Result<Vec<_>, _>>().map_err(ProjectImportError::into_report)?;

    Ok((workspace_file, project_files))
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum ProjectImportError {
    #[error("Couldn't load file {1}: {0}")]
    IoError(std::io::Error, PathBuf),
    #[error("Couldn't parse project file {1}")]
    ParsingError(super::ParsingError, PathBuf),
    #[error("File not contained in workspace")]
    FileNotInWorkspace {
        file_path: PathBuf,
        workspace_root: PathBuf,
    },
    #[error("Error importing a task")]
    #[diagnostic()]
    ErrorImportingTasks(#[from] TaskImportError),
}

impl ProjectImportError {
    pub fn into_report(self) -> miette::Report {
        match self {
            ProjectImportError::ParsingError(inner, _) => miette::Report::new(inner),
            ProjectImportError::ErrorImportingTasks(TaskImportError::ParsingError(inner)) => {
                miette::Report::new(inner)
            }
            other => miette::Report::new(other),
        }
    }
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

    // TODO: probably also need to make sure none of these are canonically out of our workspace...
    glob::glob(&glob)
        .expect("Project path patterns broke innit")
        .map(|r| r.expect("something wrong with the result"))
        .collect()
}

fn find_workspace_file(mut current_path: PathBuf) -> Option<PathBuf> {
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

// TODO: ideally want this to reference the place where we failed to load...
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[diagnostic(help("There was a problem loading a referenced task."))]
pub enum TaskImportError {
    #[error("Couldn't load {0}: {1}")]
    IoError(PathBuf, std::io::Error),
    #[error("Couldn't parse task file")]
    ParsingError(super::ParsingError),
    #[error("File not contained in workspace")]
    FileNotInWorkspace {
        file_path: PathBuf,
        workspace_root: PathBuf,
    },
}

fn import_tasks(
    mut project_file: ProjectFile,
    workspace_root: &Path,
) -> Result<ProjectFile, TaskImportError> {
    let mut imports = std::mem::take(&mut project_file.config.tasks.imports);
    while let Some(import) = imports.pop() {
        let path = match import.strip_prefix('/') {
            Some(relative_path) => workspace_root.join(relative_path),
            None => workspace_root.join(project_file.project_root.join(import)),
        };

        let Ok(relative_path) = path.strip_prefix(workspace_root) else {
            return Err(TaskImportError::FileNotInWorkspace { file_path: path, workspace_root: workspace_root.to_owned() });
        };

        // TODO: for safety, need to make sure this is still a subpath of workspace_root.
        // possibly also need to canonicalise it...?
        let path = path
            .canonicalize()
            .map_err(|e| TaskImportError::IoError(relative_path.to_owned(), e))?;

        if !path.starts_with(workspace_root) {
            return Err(TaskImportError::FileNotInWorkspace {
                file_path: path,
                workspace_root: workspace_root.to_owned(),
            });
        }

        let parsed = parse_task_file(
            &path,
            &std::fs::read_to_string(&path)
                .map_err(|e| TaskImportError::IoError(relative_path.to_owned(), e))?,
        )
        .map_err(TaskImportError::ParsingError)?;

        imports.extend(parsed.config.imports);

        project_file.config.tasks.tasks.extend(parsed.config.tasks);
    }

    Ok(project_file)
}
