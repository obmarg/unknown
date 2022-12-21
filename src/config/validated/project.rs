use crate::config::spanned::Spanned;

use super::{super::paths::ValidPath, tasks};

#[derive(Debug)]
pub struct ProjectDefinition {
    pub project: String,
    pub dependencies: Vec<Spanned<ValidPath>>,
    pub tasks: tasks::TaskBlock,
}
