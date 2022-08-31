mod project;
mod tasks;

pub use project::ProjectNode;

#[cfg(test)]
mod tests;

// TODO: flesh this out
#[derive(Debug)]
struct ParsingError(knuffel::Error);

fn project_from_str(s: &str) -> Result<Vec<ProjectNode>, ParsingError> {
    // TODO: parse a project from a string.
    knuffel::parse("file", s).map_err(ParsingError)
}
