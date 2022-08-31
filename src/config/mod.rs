mod project;
mod tasks;
mod workspace;

pub use project::ProjectNode;
pub use workspace::WorkspaceFile;

#[cfg(test)]
mod tests;

// TODO: flesh this out
struct ParsingError(knuffel::Error);

fn project_from_str(s: &str) -> Result<Vec<ProjectNode>, ParsingError> {
    // TODO: parse a project from a string.
    knuffel::parse("file", s).map_err(ParsingError)
}

fn workspace_from_str(s: &str) -> Result<WorkspaceFile, ParsingError> {
    // TODO: parse a project from a string.
    knuffel::parse("file", s).map_err(ParsingError)
}
