use super::{super::paths::ValidPath, tasks};

#[derive(Debug)]
pub struct ProjectDefinition {
    pub project: String,
    pub dependencies: Vec<ValidPath>,
    pub tasks: tasks::TaskBlock,
}
