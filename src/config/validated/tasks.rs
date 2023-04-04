use serde::Serialize;

use crate::config::{ConfigSource, Glob};

use super::super::{paths::ValidPath, spanned::Spanned};

#[derive(Debug, Default, Serialize)]
pub struct TaskBlock {
    pub(in crate::config) imports: Vec<ValidPath>,

    pub tasks: Vec<TaskDefinition>,
}

#[derive(Debug, Serialize)]
pub struct TaskDefinition {
    pub name: String,

    pub commands: Vec<String>,

    pub requires: Vec<TaskRequires>,

    pub input_blocks: Vec<InputBlock>,

    pub source: ConfigSource,
}

#[derive(Debug, Serialize)]
pub struct TaskRequires {
    pub task: Spanned<String>,
    pub target: Option<Spanned<TargetSelector>>,
}

#[derive(Debug, Serialize)]
pub struct InputBlock {
    pub paths: Vec<Glob>,
    pub env_vars: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub enum TargetSelector {
    CurrentProject,
    DependenciesOfCurrent,
    SpecificDependency(Spanned<SpecificProjectSelector>),
}

#[derive(Clone, Debug, Serialize)]
pub enum SpecificProjectSelector {
    ByName(String),
    ByPath(ValidPath),
}
