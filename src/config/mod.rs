mod config_source;
mod glob;
mod loader;
mod parsing;
mod paths;
mod spanned;
mod validated;

use self::paths::ConfigPath;
pub use self::{
    config_source::ConfigSource,
    glob::Glob,
    loader::{load_config_from_path, load_project_files},
    parsing::{ParsingError, Validator},
    paths::{ValidPath, WorkspaceRoot},
    validated::{project::ProjectDefinition, tasks::*, workspace::WorkspaceDefinition},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct UnvalidatedConfig {
    workspace_file: UnvalidatedWorkspaceFile,
    project_files: Vec<UnvalidatedProjectFile>,
}

impl UnvalidatedConfig {
    pub fn workspace_root(&self) -> &WorkspaceRoot {
        &self.workspace_file.workspace_root
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Encountered some errors when validating config files")]
struct ConfigValidationError(#[related] Vec<miette::Report>);

#[derive(Debug)]
pub struct ValidConfig {
    pub workspace_file: WorkspaceFile,
    pub project_files: Vec<ValidProjectFile>,
}

#[derive(Debug)]
pub struct UnvalidatedWorkspaceFile {
    pub workspace_root: WorkspaceRoot,
    config: parsing::WorkspaceDefinition,
    source: ConfigSource,
}

#[derive(Debug)]
pub struct WorkspaceFile {
    pub workspace_root: WorkspaceRoot,
    pub config: validated::WorkspaceDefinition,
    pub source: ConfigSource,
}

#[derive(Debug)]
pub struct UnvalidatedProjectFile {
    pub project_root: ValidPath,
    config: parsing::ProjectDefinition,
    source: ConfigSource,
}

impl UnvalidatedProjectFile {
    pub fn unvalidated_dependency_paths(&self) -> impl Iterator<Item = &ConfigPath> {
        self.config.dependencies.projects.iter().map(|p| p.as_ref())
    }
}

#[derive(Debug)]
pub struct ValidProjectFile {
    pub project_root: ValidPath,
    pub config: validated::ProjectDefinition,
    pub source: ConfigSource,
}

#[derive(Debug)]
pub struct TaskFile {
    pub config: validated::TaskBlock,
}
