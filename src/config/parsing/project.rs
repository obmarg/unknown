use crate::config::spanned::Spanned;

use super::{super::paths::ConfigPath, tasks};

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
    pub(in crate::config) projects: Vec<Spanned<ConfigPath>>,
}
