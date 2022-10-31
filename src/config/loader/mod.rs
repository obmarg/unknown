use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;

use super::{
    parsing::{parse_project_file, parse_task_file, parse_workspace_file, ParsingError},
    paths::{NormalisedPath, PathError, WorkspaceRoot},
    ProjectFile, WorkspaceFile,
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
    let config = parse_workspace_file(
        workspace_path.as_ref(),
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )?;
    let workspace_root = WorkspaceRoot::new(workspace_path.parent().unwrap());
    let workspace_file = WorkspaceFile {
        workspace_root: workspace_root.clone(),
        config,
    };

    let project_paths = workspace_file
        .config
        .project_paths
        .iter()
        .map(|g| g.clone().into_inner())
        .collect::<Vec<_>>();

    let project_file_paths = find_project_files(&workspace_root, &project_paths);

    let project_files = project_file_paths
        .iter()
        .map(|project_file_path| -> Result<_, miette::Report> {
            let config_text = std::fs::read_to_string(&project_file_path.full_path())
                .expect("couldn't read project file");
            let mut config = parse_project_file(project_file_path.as_subpath(), &config_text)
                .map_err(miette::Report::new)?;

            let project_root = project_file_path
                .parent()
                .expect("a file path to always have a parent");

            config
                .validate_and_normalise(&workspace_root, &project_root)
                .map_err(|e| {
                    miette::Report::new(e).with_source_code(miette::NamedSource::new(
                        project_file_path.as_subpath(),
                        config_text,
                    ))
                })?;

            let project_file = ProjectFile {
                project_root,
                config,
            };

            import_tasks(project_file).map_err(miette::Report::new)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((workspace_file, project_files))
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("Couldn't find a workspace.kdl")]
#[diagnostic(help("nabs requires a workspace.kdl in the current directory or a parent directory"))]
struct MissingWorkspaceFile;

fn find_project_files(root: &WorkspaceRoot, project_paths: &[Glob]) -> Vec<NormalisedPath> {
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
        .map(|d| {
            root.normalise_subpath(Utf8Path::from_path(d.path()).expect("a utf8 path"))
                .expect("to be able to normalise a path found via WalkBuilder")
        })
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
}

fn import_tasks(mut project_file: ProjectFile) -> Result<ProjectFile, TaskImportError> {
    let mut imports = std::mem::take(&mut project_file.config.tasks.imports);

    while let Some(import) = imports.pop() {
        let task_path = import
            .into_normalised()
            .expect("paths to be normalised before calling import_task");

        let mut config = parse_task_file(
            task_path.as_subpath(),
            &std::fs::read_to_string(task_path.full_path())
                .map_err(|e| TaskImportError::IoError(task_path.as_subpath().to_owned(), e))?,
        )
        .map_err(TaskImportError::ParsingError)?;

        config
            .validate_and_normalise(&task_path.parent().unwrap())
            .map_err(|e| {
                // Actually not sure how to handle this...
                // Ideally want nice miette errors highlighting the line, but that's
                // going to be a tiny bit of work...
                todo!("error handling")
            })?;

        imports.extend(config.imports.into_iter());

        project_file.config.tasks.tasks.extend(config.tasks);
    }

    Ok(project_file)
}
