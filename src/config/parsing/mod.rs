use camino::{Utf8Path, Utf8PathBuf};

mod project;
mod tasks;
mod validation;
mod workspace;

pub(super) use self::{project::ProjectDefinition, tasks::*, workspace::WorkspaceDefinition};

pub use validation::Validator;

use super::ConfigSource;

#[derive(Debug, thiserror::Error)]
#[error("Error parsing {1}")]
pub struct ParsingError(pub(super) knuffel::Error, pub(super) Utf8PathBuf);

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

pub fn parse_project_file(config: &ConfigSource) -> Result<ProjectDefinition, ParsingError> {
    let filename = config.filename();
    let config = knuffel::parse::<ProjectDefinition>(filename, config.contents())
        .map_err(|e| ParsingError(e, filename.to_owned().into()))?;

    // TODO: At some point want to validate the data in the file.
    // e.g. names can't have commas or slashes in them etc.

    Ok(config)
}

pub fn parse_workspace_file(
    path: &Utf8Path,
    contents: &str,
) -> Result<WorkspaceDefinition, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename"),
        contents,
    )
    .map_err(|e| {
        let filename = Utf8PathBuf::from(path.file_name().unwrap());
        ParsingError(e, filename)
    })?;

    // TODO: At some point want to validate the data in the file.
    // e.g. names can't have commas or slashes in them etc.

    Ok(config)
}

pub fn parse_task_file(path: &Utf8Path, contents: &str) -> Result<TaskBlock, ParsingError> {
    let config = knuffel::parse(
        path.file_name()
            .expect("workspace file path to have a filename"),
        contents,
    )
    .map_err(|e| ParsingError(e, path.to_owned()))?;

    // TODO: At some point want to validate the data in the file.
    // e.g. names can't have commas or slashes in them etc.

    Ok(config)
}
