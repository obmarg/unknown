use super::{
    paths::{ConfigPath, ConfigPathValidationError, ValidPath},
    tasks,
};

#[derive(knuffel::Decode, Debug)]
pub struct ProjectDefinition {
    #[knuffel(child, unwrap(argument))]
    pub project: String,

    #[knuffel(child, default)]
    pub dependencies: DependencyBlock,

    #[knuffel(child, default)]
    pub tasks: tasks::TaskBlock,
}

#[derive(knuffel::Decode, Debug, Default)]
pub struct DependencyBlock {
    #[knuffel(children(name = "project"), unwrap(argument))]
    pub projects: Vec<ConfigPath>,
    // #[knuffel(children(name = "import"))]
    // pub imports: Vec<DependencyImport>,
}

impl ProjectDefinition {
    pub fn validate_and_normalise(
        &mut self,
        project_path: &ValidPath,
    ) -> Result<(), ConfigPathValidationError> {
        for path in &mut self.dependencies.projects {
            path.validate_relative_to(project_path)?;
        }
        self.tasks.validate_and_normalise(project_path)?;
        Ok(())
    }
}

// #[derive(knuffel::Decode, Debug)]
// pub struct DependencyImport {
//     #[knuffel(argument)]
//     path: String,
// }
