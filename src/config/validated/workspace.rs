use serde::Serialize;

use super::super::Glob;

#[derive(knuffel::Decode, Debug, Serialize)]
pub struct WorkspaceDefinition {
    pub name: String,
    pub project_paths: Vec<Glob>,
}
