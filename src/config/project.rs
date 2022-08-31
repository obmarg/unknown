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
    #[knuffel(children(name = "project"), unwrap(argument))]
    pub projects: Vec<String>,

    #[knuffel(children(name = "path"), unwrap(argument))]
    pub paths: Vec<String>,

    #[knuffel(children(name = "import"))]
    pub imports: Vec<DependencyImport>,
}

#[derive(knuffel::Decode, Debug)]
pub struct DependencyImport {
    #[knuffel(argument)]
    path: String,
}
