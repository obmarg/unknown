use serde::Serialize;

use crate::config::spanned::Spanned;

use super::{super::paths::ValidPath, tasks};

#[derive(Debug, Serialize)]
pub struct ProjectDefinition {
    pub project: String,
    pub dependencies: Vec<Spanned<ValidPath>>,
    pub tasks: tasks::TaskBlock,
}
