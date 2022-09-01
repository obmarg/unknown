use super::tasks;

#[derive(knuffel::Decode, Debug)]
pub struct ProjectDefinition {
    #[knuffel(child, unwrap(argument))]
    pub project: String,

    #[knuffel(child, default)]
    pub dependencies: DependencyBlock,

    #[knuffel(child, default)]
    pub tasks: tasks::TaskBlock,
}

#[derive(knuffel::Decode, Debug)]
pub struct Project {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug, Default)]
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
