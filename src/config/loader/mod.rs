use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;

use super::{
    parsing::{parse_project_file, parse_task_file, parse_workspace_file, Validator},
    paths::{RelativePath, ValidPath, WorkspaceRoot},
    UnvalidatedConfig, UnvalidatedProjectFile, UnvalidatedWorkspaceFile, ValidConfig,
    ValidProjectFile, WorkspaceFile,
};

#[cfg(test)]
mod tests;

pub fn load_unvalidated_config_from_path(
    current_path: Utf8PathBuf,
) -> Result<UnvalidatedConfig, miette::Report> {
    let current_path = current_path
        .canonicalize_utf8()
        .expect("to be able to canonicalise current path");

    let workspace_path = find_workspace_file(current_path).ok_or(MissingWorkspaceFile)?;
    let config = parse_workspace_file(
        workspace_path.as_ref(),
        &std::fs::read_to_string(&workspace_path).expect("couldn't read workspace file"),
    )?;
    let workspace_root = WorkspaceRoot::new(workspace_path.parent().unwrap());
    let workspace_file = UnvalidatedWorkspaceFile {
        workspace_root: workspace_root.clone(),
        config,
    };

    let project_paths = workspace_file
        .config
        .project_paths
        .iter()
        .map(|g| g.clone().into_inner())
        .collect::<Vec<_>>();

    let project_file_paths =
        find_project_files(&workspace_root, workspace_root.as_ref(), &project_paths);

    let project_files = project_file_paths
        .iter()
        .map(load_project_file)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(UnvalidatedConfig {
        workspace_file,
        project_files,
    })
}

pub fn load_config_from_path(current_path: Utf8PathBuf) -> Result<ValidConfig, miette::Report> {
    let unvalidated = load_unvalidated_config_from_path(current_path)?;
    let mut validator = Validator::new(unvalidated.workspace_root().clone());

    let mut config = validator.validate_config(unvalidated)?;

    for project in &mut config.project_files {
        import_tasks(project, &mut validator)?
    }

    validator.ok()?;

    Ok(config)
}

pub fn load_project_files(
    root: &WorkspaceRoot,
    path: &RelativePath,
    project_paths: &[Glob],
) -> Result<Vec<UnvalidatedProjectFile>, miette::Report> {
    find_project_files(root, &path.to_absolute(), project_paths)
        .iter()
        .map(load_project_file)
        .collect()
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("Couldn't find a workspace.kdl")]
#[diagnostic(help("nabs requires a workspace.kdl in the current directory or a parent directory"))]
struct MissingWorkspaceFile;

fn find_project_files(
    workspace_root: &WorkspaceRoot,
    path: &Utf8Path,
    project_paths: &[Glob],
) -> Vec<ValidPath> {
    let mut glob_builder = GlobSetBuilder::new();
    if project_paths.is_empty() {
        glob_builder.add(Glob::new("**").unwrap());
    }
    for path in project_paths {
        glob_builder.add(path.clone());
    }
    let glob_set = glob_builder.build().expect("to be able to build globset");

    WalkBuilder::new(path)
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
                let relative_folder = file_path
                    .parent()
                    .unwrap()
                    .strip_prefix(workspace_root)
                    .unwrap();
                return glob_set.is_match(relative_folder);
            }
            false
        })
        .map(|d| {
            workspace_root
                .normalise_absolute(Utf8Path::from_path(d.path()).expect("a utf8 path"))
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

fn load_project_file(
    project_file_path: &ValidPath,
) -> Result<UnvalidatedProjectFile, miette::Report> {
    let source_text =
        std::fs::read_to_string(project_file_path.full_path()).expect("couldn't read project file");
    let config = parse_project_file(project_file_path.as_subpath(), &source_text)
        .map_err(miette::Report::new)?;

    let project_root = project_file_path
        .parent()
        .expect("a file path to always have a parent");

    Ok(UnvalidatedProjectFile {
        project_root,
        config,
        source_text,
        project_file_path: project_file_path.clone(),
    })
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

pub(super) fn import_tasks(
    project_file: &mut ValidProjectFile,
    validator: &mut Validator,
) -> Result<(), miette::Report> {
    let mut imports = std::mem::take(&mut project_file.config.tasks.imports);

    while let Some(task_path) = imports.pop() {
        let task_file_contents = std::fs::read_to_string(task_path.full_path())
            .map_err(|e| TaskImportError::IoError(task_path.as_subpath().to_owned(), e))?;

        let config: super::parsing::TaskBlock =
            parse_task_file(task_path.as_subpath(), &task_file_contents)
                .map_err(TaskImportError::ParsingError)?;

        let source_code =
            super::parsing::SourceCode::new(task_path.as_subpath(), task_file_contents);

        let config = validator.validate_tasks(config, &task_path.parent().unwrap(), &source_code);

        if let Some(config) = config {
            imports.extend(config.imports.into_iter());
            project_file.config.tasks.tasks.extend(config.tasks);
        }
    }

    Ok(())
}
