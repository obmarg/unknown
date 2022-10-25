use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;

use super::{
    parse_project_file, parse_task_file, parse_workspace_file, ProjectFile, WorkspaceFile,
};

#[cfg(test)]
mod tests;

pub fn load_config_from_path(
    current_path: Utf8PathBuf,
) -> Result<(WorkspaceFile, Vec<ProjectFile>), miette::Report> {
    let current_path = current_path
        .canonicalize_utf8()
        .expect("to be able to canonicalise current path");

    let workspace_path = find_workspace_file(current_path).ok_or(MissingWorkspaceFile)?;
    let workspace_file = parse_workspace_file(
        &workspace_path,
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )?;

    let workspace_root = workspace_path.parent().unwrap();
    let project_paths = workspace_file
        .config
        .project_paths
        .iter()
        .map(|g| g.clone().into_inner())
        .collect::<Vec<_>>();

    let project_config_paths = find_project_files(workspace_root, &project_paths);

    let project_files = project_config_paths
        .iter()
        .map(|path| {
            let canonical_path = path
                .canonicalize_utf8()
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
    #[error("Couldn't parse project file {1}")]
    ParsingError(super::ParsingError, Utf8PathBuf),
    #[error("File not contained in workspace")]
    FileNotInWorkspace {
        file_path: Utf8PathBuf,
        workspace_root: Utf8PathBuf,
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

fn find_project_files(root: &Utf8Path, project_paths: &[Glob]) -> Vec<Utf8PathBuf> {
    let mut glob_builder = GlobSetBuilder::new();
    if project_paths.is_empty() {
        glob_builder.add(Glob::new("**").unwrap());
    }
    for path in project_paths {
        glob_builder.add(path.clone());
    }
    let glob_set = glob_builder.build().expect("to be able to build globset");

    WalkBuilder::new(root)
        .hidden(false)
        .build()
        .filter_map(|f| f.ok())
        .filter(|entry| {
            let file_path = entry.path();
            if file_path.is_file()
                && file_path
                    .file_name()
                    .map(|p| p == "project.kdl")
                    .unwrap_or_default()
            {
                let relative_folder = file_path.parent().unwrap().strip_prefix(root).unwrap();
                return glob_set.is_match(relative_folder);
            }
            false
        })
        .map(|d| Utf8Path::from_path(d.path()).unwrap().to_owned())
        .collect()
}

fn find_workspace_file(mut current_path: Utf8PathBuf) -> Option<Utf8PathBuf> {
    while current_path.parent().is_some() {
        current_path.push("workspace.kdl");
        if current_path.exists() {
            return Some(
                current_path
                    .canonicalize_utf8()
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
    IoError(Utf8PathBuf, std::io::Error),
    #[error("Couldn't parse task file")]
    ParsingError(super::ParsingError),
    #[error("File not contained in workspace")]
    FileNotInWorkspace {
        file_path: Utf8PathBuf,
        workspace_root: Utf8PathBuf,
    },
}

fn import_tasks(
    mut project_file: ProjectFile,
    workspace_root: &Utf8Path,
) -> Result<ProjectFile, TaskImportError> {
    let mut imports = project_file
        .config
        .tasks
        .imports
        .drain(0..)
        .map(|path| (path, project_file.project_root.clone()))
        .collect::<Vec<_>>();

    while let Some((import, relative_to)) = imports.pop() {
        let path = match import.strip_prefix('/') {
            Some(relative_path) => workspace_root.join(relative_path),
            None => workspace_root.join(relative_to.join(import)),
        };

        let Ok(relative_path) = path.strip_prefix(workspace_root) else {
            return Err(TaskImportError::FileNotInWorkspace { file_path: path, workspace_root: workspace_root.to_owned() });
        };

        // TODO: for safety, need to make sure this is still a subpath of workspace_root.
        // possibly also need to canonicalise it...?
        let path = path
            .canonicalize_utf8()
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

        imports.extend(parsed.config.imports.into_iter().map(|import| {
            (
                import,
                path.parent().expect("path to have a parent").to_owned(),
            )
        }));

        project_file.config.tasks.tasks.extend(parsed.config.tasks);
    }

    Ok(project_file)
}
