use super::tasks;

#[derive(knuffel::Decode, Debug)]
pub struct ProjectFile {
    #[knuffel(child, unwrap(argument))]
    pub project: String,

    #[knuffel(child)]
    pub dependencies: DependencyBlock,

    #[knuffel(child)]
    pub tasks: tasks::TaskBlock,
}

pub enum ProjectNode {
    Project(Project),
    Dependencies(DependencyBlock),
    Tasks(tasks::TaskBlock),
}

#[derive(knuffel::Decode, Debug)]
pub struct Project {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug)]
pub struct DependencyBlock {
    #[knuffel(children(name = "project"))]
    projects: Vec<ProjectDependency>,

    #[knuffel(children(name = "path"))]
    paths: Vec<PathDependency>,

    #[knuffel(children(name = "import"))]
    imports: Vec<DependencyImport>,
}

#[derive(knuffel::Decode, Debug)]
pub struct ProjectDependency {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug)]
pub struct PathDependency {
    #[knuffel(argument)]
    path: String,
}

#[derive(knuffel::Decode, Debug)]
pub struct DependencyImport {
    #[knuffel(argument)]
    path: String,
}
