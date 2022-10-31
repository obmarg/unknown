mod glob;
mod loader;
mod parsing;
mod paths;
mod project;
mod tasks;
mod workspace;

pub use self::{
    glob::Glob,
    loader::load_config_from_path,
    parsing::ParsingError,
    paths::{NormalisedPath, WorkspaceRoot},
    project::{DependencyBlock, ProjectDefinition},
    tasks::*,
    workspace::WorkspaceDefinition,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct WorkspaceFile {
    pub workspace_root: WorkspaceRoot,
    pub config: WorkspaceDefinition,
}

#[derive(Debug)]
pub struct ProjectFile {
    pub project_root: NormalisedPath,
    pub config: ProjectDefinition,
}

#[derive(Debug)]
pub struct TaskFile {
    pub config: tasks::TaskBlock,
}
