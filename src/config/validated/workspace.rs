use super::super::Glob;

#[derive(knuffel::Decode, Debug)]
pub struct WorkspaceDefinition {
    pub name: String,
    pub project_paths: Vec<Glob>,
}
