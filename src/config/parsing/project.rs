use crate::config::validated;

use super::{
    super::paths::{ConfigPath, ConfigPathValidationError, ValidPath},
    tasks, CollectResults,
};

#[derive(knuffel::Decode, Debug)]
pub struct ProjectDefinition {
    #[knuffel(child, unwrap(argument))]
    pub(super) project: String,

    #[knuffel(child, default)]
    pub(in crate::config) dependencies: DependencyBlock,

    #[knuffel(child, default)]
    pub(super) tasks: tasks::TaskBlock,
}

#[derive(knuffel::Decode, Debug, Default)]
pub struct DependencyBlock {
    #[knuffel(children(name = "project"), unwrap(argument))]
    pub(in crate::config) projects: Vec<ConfigPath>,
    // #[knuffel(children(name = "path"), unwrap(argument))]
    // pub paths: Vec<String>,
    // #[knuffel(children(name = "import"))]
    // pub imports: Vec<DependencyImport>,
}

// #[derive(knuffel::Decode, Debug)]
// pub struct DependencyImport {
//     #[knuffel(argument)]
//     path: String,
// }
