mod tasks;

#[cfg(test)]
mod tests;

// TODO: flesh this out
#[derive(Debug)]
struct ParsingError(knuffel::Error);

fn project_from_str(s: &str) -> Result<Vec<ProjectNode>, ParsingError> {
    // TODO: parse a project from a string.
    knuffel::parse("file", s).map_err(ParsingError)
}

#[derive(knuffel::Decode, Debug)]
enum ProjectNode {
    Project(Project),
    Dependencies(DependencyBlock),
    Tasks(tasks::TaskBlock),
}

#[derive(knuffel::Decode, Debug)]
struct Project {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug)]
struct DependencyBlock {
    #[knuffel(children(name = "project"))]
    projects: Vec<ProjectDependency>,

    #[knuffel(children(name = "path"))]
    paths: Vec<PathDependency>,

    #[knuffel(children(name = "import"))]
    imports: Vec<DependencyImport>,
}

#[derive(knuffel::Decode, Debug)]
struct ProjectDependency {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug)]
struct PathDependency {
    #[knuffel(argument)]
    path: String,
}

#[derive(knuffel::Decode, Debug)]
struct DependencyImport {
    #[knuffel(argument)]
    path: String,
}
