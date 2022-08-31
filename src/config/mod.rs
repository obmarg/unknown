mod project;
mod tasks;
mod workspace;

pub use project::ProjectFile;
pub use workspace::WorkspaceFile;

#[cfg(test)]
mod tests;

// TODO: flesh this out
pub struct ParsingError(knuffel::Error);

impl ParsingError {
    pub fn into_report(self) -> miette::Report {
        miette::Report::new(self.0)
    }
}

pub fn project_from_str(s: &str) -> Result<ProjectFile, ParsingError> {
    knuffel::parse("file", s).map_err(ParsingError)
}

pub fn workspace_from_str(s: &str) -> Result<WorkspaceFile, ParsingError> {
    knuffel::parse("file", s).map_err(ParsingError)
}
