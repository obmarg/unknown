mod glob;
mod loader;
mod project;
mod tasks;
mod workspace;

use camino::{Utf8Path, Utf8PathBuf};

pub use self::{
    glob::Glob,
    loader::load_config_from_path,
    project::{DependencyBlock, ProjectDefinition},
    tasks::*,
    workspace::WorkspaceDefinition,
};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
#[error("Error parsing {1}")]
pub struct ParsingError(knuffel::Error, Utf8PathBuf);

impl miette::Diagnostic for ParsingError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.0.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.0.source_code()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.0.labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        self.0.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        self.0.diagnostic_source()
    }
}

#[derive(Debug)]
pub struct ProjectFile {
    pub project_root: Utf8PathBuf,
    pub config: ProjectDefinition,
}

pub fn parse_project_file(path: &Utf8Path, contents: &str) -> Result<ProjectFile, ParsingError> {
    let config = knuffel::parse::<ProjectDefinition>(
        path.file_name()
            .expect("project file path to have a filename"),
        contents,
    )
    .map_err(|e| ParsingError(e, path.to_owned()))?;

    // TODO: At some point want to validate the data in the file.
    // e.g. names can't have commas or slashes in them etc.

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
    pub workspace_root: Utf8PathBuf,
    pub config: WorkspaceDefinition,
}

pub fn parse_workspace_file(
    path: &Utf8Path,
    contents: &str,
) -> Result<WorkspaceFile, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename"),
        contents,
    )
    .map_err(|e| {
        let filename = Utf8PathBuf::from(path.file_name().unwrap());
        ParsingError(e, filename)
    })?;

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

pub fn parse_task_file(path: &Utf8Path, contents: &str) -> Result<TaskFile, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename"),
        contents,
    )
    .map_err(|e| ParsingError(e, path.to_owned()))?;

    Ok(TaskFile { config })
}
