use std::str::FromStr;

use crate::config::Glob;

use super::super::{
    paths::{ConfigPath, ConfigPathValidationError, ValidPath},
    spanned::Spanned,
};

#[derive(Debug, Default)]
pub struct TaskBlock {
    // TODO: what should this be...?
    pub(in crate::config) imports: Vec<ValidPath>,

    pub tasks: Vec<TaskDefinition>,
}

#[derive(Debug)]
pub struct TaskDefinition {
    pub name: String,

    pub commands: Vec<String>,

    // TODO: ditch this
    pub dependencies: Vec<TaskDependency>,

    pub requires: Vec<TaskRequires>,

    pub input_blocks: Vec<InputBlock>,
}

#[derive(Debug)]
pub struct TaskRequires {
    pub task: String,
    pub target: TargetSelector,
}

#[derive(Debug)]
pub struct TaskDependency {
    pub task: String,

    // TODO: This feels like a shit name, come up with something better.
    // for_ancestors?  for_parents?
    // run_for_parents?
    pub for_project_deps: Option<bool>,

    pub include_this_package: Option<bool>,
}

#[derive(Debug)]
pub struct InputBlock {
    pub paths: Vec<Glob>,
    pub env_vars: Vec<String>,
    pub commands: Vec<String>,
}

// TargetSelector maybe?
#[derive(Clone, Debug)]
pub enum TargetSelector {
    Project(Spanned<TargetAnchor>),
    ProjectWithDependencies(Spanned<TargetAnchor>),
    JustDependencies(Spanned<TargetAnchor>),
}

#[derive(Clone, Debug)]
pub enum TargetAnchor {
    CurrentProject,
    ProjectByName(String),
    ProjectByPath(ValidPath),
}
