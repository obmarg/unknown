use crate::config::spanned::Spanned;

use super::{
    super::paths::{ConfigPath, ConfigPathValidationError, ValidPath},
    tasks,
};

#[derive(Debug)]
pub struct ProjectDefinition {
    pub project: String,
    pub dependencies: Vec<ValidPath>,
    pub tasks: tasks::TaskBlock,
}
