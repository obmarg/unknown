#[derive(knuffel::Decode, Debug)]
pub struct WorkspaceFile {
    #[knuffel(child, unwrap(argument))]
    pub name: String,

    #[knuffel(children(name = "project_path"), unwrap(argument))]
    pub project_paths: Vec<String>,
}
