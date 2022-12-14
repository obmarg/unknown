use super::super::Glob;

#[derive(knuffel::Decode, Debug)]
pub struct WorkspaceDefinition {
    #[knuffel(child, unwrap(argument))]
    pub name: String,

    #[knuffel(children(name = "project_path"), unwrap(argument))]
    pub project_paths: Vec<Glob>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("Workspace failed validation")]
pub enum WorkspaceValidationError {}
