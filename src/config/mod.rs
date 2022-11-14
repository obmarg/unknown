mod glob;
mod loader;
mod parsing;
mod paths;
mod project;
mod tasks;
mod workspace;

use self::paths::ConfigPath;
pub use self::{
    glob::Glob,
    loader::{load_config_from_path, load_project_files},
    parsing::ParsingError,
    paths::{ValidPath, WorkspaceRoot},
    project::{DependencyBlock, ProjectDefinition},
    tasks::*,
    workspace::WorkspaceDefinition,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct UnvalidatedConfig {
    workspace_file: WorkspaceFile,
    project_files: Vec<UnvalidatedProjectFile>,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Encountered some errors when validating config files")]
struct ConfigValidationError {
    #[related]
    errors: Vec<miette::Report>,
}

impl UnvalidatedConfig {
    pub fn validate(self) -> Result<ValidConfig, miette::Report> {
        let mut project_files = Vec::with_capacity(self.project_files.len());
        let mut errors = Vec::new();

        for project_file in self.project_files {
            match project_file.validate() {
                Ok(mut project_file) => {
                    project_file.import_tasks()?;
                    project_files.push(project_file);
                }
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(ConfigValidationError { errors }.into());
        }

        Ok(ValidConfig {
            workspace_file: self.workspace_file,
            project_files,
        })
    }
}

#[derive(Debug)]
pub struct ValidConfig {
    pub workspace_file: WorkspaceFile,
    pub project_files: Vec<ValidProjectFile>,
}

#[derive(Debug)]
pub struct WorkspaceFile {
    pub workspace_root: WorkspaceRoot,
    pub config: WorkspaceDefinition,
}

#[derive(Debug)]
pub struct UnvalidatedProjectFile {
    pub project_root: ValidPath,
    project_file_path: ValidPath,
    config: ProjectDefinition,
    source_text: String,
}

impl UnvalidatedProjectFile {
    pub fn unvalidated_dependency_paths(&self) -> impl Iterator<Item = &ConfigPath> {
        self.config.dependencies.projects.iter()
    }

    pub fn validate(mut self) -> Result<ValidProjectFile, miette::Report> {
        self.config
            .validate_and_normalise(&self.project_root)
            .map_err(|e| {
                miette::Report::new(e).with_source_code(miette::NamedSource::new(
                    self.project_file_path.as_subpath(),
                    self.source_text,
                ))
            })?;

        Ok(ValidProjectFile {
            project_root: self.project_root,
            config: self.config,
        })
    }
}

#[derive(Debug)]
pub struct ValidProjectFile {
    pub project_root: ValidPath,
    pub config: ProjectDefinition,
}

impl ValidProjectFile {
    pub fn import_tasks(&mut self) -> Result<(), miette::Report> {
        loader::import_tasks(self)
    }
}

#[derive(Debug)]
pub struct TaskFile {
    pub config: tasks::TaskBlock,
}
