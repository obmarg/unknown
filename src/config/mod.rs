mod loader;
mod project;
mod tasks;
mod workspace;

use std::path::{Path, PathBuf};

pub use self::{
    loader::load_config_from_cwd, project::ProjectDefinition, tasks::*,
    workspace::WorkspaceDefinition,
};

#[cfg(test)]
mod tests;

// TODO: flesh this out
pub struct ParsingError(knuffel::Error);

impl ParsingError {
    pub fn into_report(self) -> miette::Report {
        miette::Report::new(self.0)
    }
}

#[derive(Debug)]
pub struct ProjectFile {
    pub project_root: PathBuf,
    pub config: ProjectDefinition,
}

pub fn parse_project_file(path: &Path, contents: &str) -> Result<ProjectFile, ParsingError> {
    let config = knuffel::parse::<ProjectDefinition>(
        path.file_name()
            .expect("project file path to have a filename")
            .to_string_lossy()
            .as_ref(),
        contents,
    )
    .map_err(ParsingError)?;

    Ok(ProjectFile {
        project_root: path
            .parent()
            .expect("project file path to have a parent")
            .to_owned(),
        config,
    })
}

#[derive(Debug)]
pub struct WorkspaceFile {
    pub workspace_root: PathBuf,
    pub config: WorkspaceDefinition,
}

pub fn parse_workspace_file(path: &Path, contents: &str) -> Result<WorkspaceFile, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename")
            .to_string_lossy()
            .as_ref(),
        contents,
    )
    .map_err(ParsingError)?;

    Ok(WorkspaceFile {
        workspace_root: path
            .parent()
            .expect("workspace file path to have a parent")
            .to_owned(),
        config,
    })
}

#[derive(Debug)]
pub struct TaskFile {
    pub config: tasks::TaskBlock,
}

pub fn parse_task_file(path: &Path, contents: &str) -> Result<TaskFile, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename")
            .to_string_lossy()
            .as_ref(),
        contents,
    )
    .map_err(ParsingError)?;

    Ok(TaskFile { config })
}
