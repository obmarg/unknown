// #[derkve(knuffel::Decode, Debug)]
// pub enum WorkspaceNode {
//     // Name(WorkspaceName),
//     // ProjectPaths(ProjectPath),
// }

#[derive(knuffel::Decode, Debug)]
pub struct WorkspaceFile {
    #[knuffel(child, unwrap(argument))]
    name: String,

    #[knuffel(children(name = "project_path"), unwrap(argument))]
    project_paths: Vec<String>,
}
