// TODO: Consider having a validator type that contains a list of errors
// and other neccesary context, then have all the validation done by that.
// Might be neater than trying to be all object oriented about it...

use crate::{
    config::{
        parsing, validated, ConfigSource, UnvalidatedConfig, UnvalidatedProjectFile,
        UnvalidatedWorkspaceFile, ValidConfig, ValidPath, ValidProjectFile, WorkspaceFile,
        WorkspaceRoot,
    },
    diagnostics::{CollectResults, ConfigError, DynDiagnostic},
};

pub struct Validator {
    workspace_root: WorkspaceRoot,
    errors: Vec<DynDiagnostic>,
}

// TODO: Ensure the public API of this makes sense once I'm done.
impl Validator {
    pub fn new(workspace_root: WorkspaceRoot) -> Self {
        Validator {
            workspace_root,
            errors: vec![],
        }
    }

    pub fn ok(&mut self) -> Result<(), ConfigError> {
        if self.errors.is_empty() {
            return Ok(());
        }
        Err(ConfigError {
            errors: std::mem::take(&mut self.errors),
        })
    }

    pub fn validate_config(
        &mut self,
        config: UnvalidatedConfig,
    ) -> Result<ValidConfig, ConfigError> {
        let workspace_file = self.validate_workspace_file(config.workspace_file);

        let project_files = config
            .project_files
            .into_iter()
            .map(|project_file| self.validate_project_file(project_file))
            .collect::<Option<Vec<_>>>();

        self.ok()?;

        let (workspace_file, project_files) = workspace_file
            .zip(project_files)
            .expect("validation errors if theres any nones here");

        // TODO: This may need to do some final validation
        // to make sure all the task references line up and stuff...

        Ok(ValidConfig {
            workspace_file,
            project_files,
        })
    }

    fn validate_workspace_file(
        &mut self,
        workspace: UnvalidatedWorkspaceFile,
    ) -> Option<WorkspaceFile> {
        Some(WorkspaceFile {
            workspace_root: workspace.workspace_root,
            config: validated::WorkspaceDefinition {
                name: workspace.config.name,
                project_paths: workspace.config.project_paths,
            },
            source: workspace.source,
        })
    }

    fn validate_project_file(&mut self, file: UnvalidatedProjectFile) -> Option<ValidProjectFile> {
        let config =
            self.validate_project_definition(file.config, &file.project_root, &file.source)?;

        Some(ValidProjectFile {
            project_root: file.project_root,
            config,
            source: file.source,
        })
    }

    pub fn validate_project_definition(
        &mut self,
        project: parsing::ProjectDefinition,
        project_path: &ValidPath,
        config_source: &ConfigSource,
    ) -> Option<validated::ProjectDefinition> {
        let dependencies = project
            .dependencies
            .projects
            .into_iter()
            .map(|p| p.validate_relative_to(project_path))
            .collect_results();

        let tasks = self.validate_tasks(project.tasks, project_path, config_source);

        let (dependencies, tasks) = self.record_errors(dependencies, config_source).zip(tasks)?;

        // let tasks = project.tasks.validate(project_path)?;

        Some(validated::ProjectDefinition {
            project: project.project,
            dependencies,
            tasks,
        })
    }

    pub fn validate_tasks(
        &mut self,
        tasks: parsing::TaskBlock,
        relative_to: &ValidPath,
        source_code: &ConfigSource,
    ) -> Option<validated::TaskBlock> {
        let imports = tasks
            .imports
            .into_iter()
            .map(|path| path.validate_relative_to(relative_to))
            .collect_results();

        let imports = self.record_errors(imports, source_code);

        let tasks = tasks
            .tasks
            .into_iter()
            .map(|task| self.validate_task(task, source_code))
            .collect::<Option<Vec<_>>>();

        let (imports, tasks) = imports.zip(tasks)?;

        Some(validated::TaskBlock { imports, tasks })
    }

    pub fn validate_task(
        &mut self,
        task: parsing::TaskDefinition,
        config_source: &ConfigSource,
    ) -> Option<validated::TaskDefinition> {
        let requires = task
            .requires
            .into_iter()
            .map(|r| r.parse(&self.workspace_root))
            .collect_results();

        let requires = self.record_errors(requires, config_source)?;

        Some(validated::TaskDefinition {
            name: task.name,
            commands: task.commands,
            requires,
            input_blocks: task.input_blocks.into_iter().map(Into::into).collect(),
            source: config_source.clone(),
        })
    }

    fn record_errors<T, E>(
        &mut self,
        res: Result<T, Vec<E>>,
        config_source: &ConfigSource,
    ) -> Option<T>
    where
        E: miette::Diagnostic + Send + Sync + 'static,
    {
        match res {
            Ok(inner) => Some(inner),
            Err(errors) => {
                for error in errors {
                    self.errors
                        .push(DynDiagnostic::new(error).with_source_code(config_source.clone()));
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_files::TestFiles;

    #[rstest]
    #[case("/library")]
    #[case("../library")]
    fn project_dependency_happy_path(#[case] path: &str) {
        let files = TestFiles::new()
            .with_file("library/project.kdl", r#"project "library"#)
            .with_file("service/project.kdl", "");
        let source = ConfigSource::new(
            "service/project.kdl",
            format!(
                r#"
                project "service"
                dependencies {{
                    project "{path}"
                }}
                "#
            ),
        );
        let project = parsing::parse_project_file(&source).unwrap();
        let mut validator = Validator::new(files.root());
        let project_path = files
            .root()
            .subpath("service/")
            .unwrap()
            .validate()
            .unwrap();

        let project = validator.validate_project_definition(project, &project_path, &source);

        validator.ok().unwrap();
        assert_eq!(project.unwrap().dependencies.len(), 1);
    }
}
