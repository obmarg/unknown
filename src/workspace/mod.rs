use std::collections::{BTreeSet, HashMap, HashSet};

use miette::SourceSpan;
use once_cell::sync::OnceCell;

use crate::{
    config::{self, ConfigSource, TargetAnchor, ValidPath, WorkspaceRoot},
    diagnostics::{CollectResults, ConfigError, DynDiagnostic},
};

use self::graph::WorkspaceGraph;

mod graph;

#[cfg(test)]
mod tests;

use camino::Utf8Path;
use globset::Glob;

pub struct Workspace {
    pub info: WorkspaceInfo,
    graph_: WorkspaceGraph,
    project_map: HashMap<ProjectRef, ProjectInfo>,
    task_map: HashMap<TaskRef, TaskInfo>,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert project_map to a BTreeMap before printing so
        // we can get a consistent order for snapshot testing
        let project_map = self
            .project_map
            .iter()
            .collect::<std::collections::BTreeMap<_, _>>();

        let task_map = self
            .task_map
            .iter()
            .collect::<std::collections::BTreeMap<_, _>>();

        f.debug_struct("Workspace")
            .field("info", &self.info)
            .field("project_map", &project_map)
            .field("task_map", &task_map)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct WorkspaceInfo {
    #[allow(unused)]
    name: String,
    pub project_paths: Vec<Glob>,
    pub root_path: WorkspaceRoot,
}

impl Workspace {
    pub fn new(workspace_file: config::WorkspaceFile) -> Self {
        let workspace_info = WorkspaceInfo {
            name: workspace_file.config.name,
            project_paths: workspace_file
                .config
                .project_paths
                .into_iter()
                .map(|g| g.into_inner())
                .collect(),
            root_path: workspace_file.workspace_root,
        };
        let graph = WorkspaceGraph::new();

        Workspace {
            graph_: WorkspaceGraph::new(),
            info: workspace_info,
            project_map: HashMap::new(),
            task_map: HashMap::new(),
        }
    }

    pub fn add_projects(
        &mut self,
        project_files: Vec<config::ValidProjectFile>,
    ) -> Result<(), ConfigError> {
        let project_paths = project_files
            .iter()
            .map(|project_file| project_file.project_root.clone())
            .collect::<HashSet<_>>();

        self.project_map.reserve(project_files.len());

        let mut tasks_to_process = Vec::new();

        for project_file in project_files {
            let project_ref = ProjectRef(project_file.project_root.clone());

            let mut dependencies = Vec::new();
            for path in project_file.config.dependencies {
                if !project_paths.contains(&path) {
                    panic!("Unknown project: {path}");
                }
                dependencies.push(ProjectRef(path));
            }

            // TODO: handle task imports
            for task in project_file.config.tasks.tasks {
                tasks_to_process.push((project_ref.clone(), task));
            }

            self.project_map.insert(
                project_ref.clone(),
                ProjectInfo {
                    name: project_file.config.project.clone(),
                    dependencies,
                    root: project_file.project_root,
                },
            );
        }

        self.graph_.add_projects(&self.project_map);

        let mut errors = Vec::new();
        for (project_ref, task) in tasks_to_process {
            let project = &self.project_map[&project_ref];
            let requires = task
                .requires
                .into_iter()
                .map(|requires| {
                    resolve_requires(requires, project_ref.lookup(self), self, &task.source)
                })
                .collect_results();

            match requires {
                Ok(requires) => {
                    self.task_map.insert(
                        TaskRef(project_ref.clone(), task.name.clone()),
                        TaskInfo {
                            project_name: project.name.clone(),
                            project: project_ref.clone(),
                            name: task.name.clone(),
                            commands: task.commands.clone(),
                            requires: requires.into_iter().flatten().collect(),
                            inputs: TaskInputs::from_config(&task.input_blocks),
                        },
                    );
                }
                Err(errs) => errors.extend(
                    errs.into_iter()
                        .map(|e| DynDiagnostic::new(e).with_source_code(task.source.clone())),
                ),
            }
        }

        if !errors.is_empty() {
            return Err(ConfigError { errors });
        }

        self.graph_.register_tasks(&self.task_map);

        Ok(())
    }

    pub fn graph(&self) -> &WorkspaceGraph {
        &self.graph_
    }

    pub fn projects(&self) -> impl Iterator<Item = &ProjectInfo> {
        self.project_map.values()
    }

    pub fn project_at_path(&self, path: impl AsRef<Utf8Path>) -> Option<&ProjectInfo> {
        let project_ref = ProjectRef(self.info.root_path.normalise_absolute(path.as_ref()).ok()?);
        self.project_map.get(&project_ref)
    }

    pub fn project_by_name(&self, name: impl AsRef<str>) -> Option<&ProjectInfo> {
        let name = name.as_ref();
        self.project_map.values().find(|p| p.name == name)
    }

    // pub fn lookup_project(&self, name: impl AsRef<str>) -> Option<&ProjectInfo> {
    //     self.project_map.get(name.as_ref())
    // }

    pub fn root_path(&self) -> &WorkspaceRoot {
        &self.info.root_path
    }

    pub fn projects_globset(&self) -> globset::GlobSet {
        let mut builder = globset::GlobSetBuilder::new();
        for glob in &self.info.project_paths {
            builder.add(glob.clone());
        }
        if self.info.project_paths.is_empty() {
            builder.add(globset::Glob::new("**").unwrap());
        }
        builder.build().unwrap()
    }
}

impl ProjectRef {
    pub fn lookup<'a>(&self, workspace: &'a Workspace) -> &'a ProjectInfo {
        // TODO: This basically assumes a ProjectRef is always valid.
        // Probably need to enforce that with types somehow or make this return an option
        &workspace.project_map[self]
    }
}

impl TaskRef {
    pub fn lookup<'a>(&self, workspace: &'a Workspace) -> &'a TaskInfo {
        workspace
            .task_map
            .get(self)
            .expect("a valid TaskRef for the given Workspace")
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub dependencies: Vec<ProjectRef>,
    // pub tasks: Vec<TaskInfo>,
    pub root: ValidPath,
}

impl ProjectInfo {
    pub fn project_ref(&self) -> ProjectRef {
        ProjectRef(self.root.clone())
    }

    pub fn tasks<'a>(&self, workspace: &'a Workspace) -> Vec<&'a TaskInfo> {
        workspace
            .graph()
            .project_tasks(&self.project_ref())
            .into_iter()
            .map(|r| r.lookup(&workspace))
            .collect()
    }

    pub fn lookup_task<'a>(&self, name: &str, workspace: &'a Workspace) -> Option<&'a TaskInfo> {
        self.tasks(workspace)
            .into_iter()
            .find(|task| task.name == name)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectRef(ValidPath);

impl ProjectRef {
    pub fn as_str(&self) -> &str {
        self.0.as_subpath().as_str()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskRef(ProjectRef, String);

impl TaskRef {
    pub fn project(&self) -> &ProjectRef {
        &self.0
    }

    pub fn task_name(&self) -> &str {
        &self.1
    }
}

impl std::fmt::Display for TaskRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.0.as_str(), self.task_name())
    }
}

/// A TaskRef that may or may not exist.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PossibleTaskRef(ProjectRef, String);

// TODO: Think about sticking this in an arc or similar rather than clone
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TaskInfo {
    pub project: ProjectRef,
    pub project_name: String,
    pub name: String,
    pub commands: Vec<String>,
    pub requires: Vec<PossibleTaskRef>,
    pub inputs: TaskInputs,
}

impl TaskInfo {
    pub fn task_ref(&self) -> TaskRef {
        TaskRef(self.project.clone(), self.name.clone())
    }
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum TaskResolutionError {
    #[diagnostic()]
    #[error("Couldn't find a project named {name}")]
    UnknownProjectByName {
        name: String,
        #[label = "Couldn't find this project"]
        span: SourceSpan,

        #[source_code]
        source_code: ConfigSource,
    },
    #[diagnostic()]
    #[error("Couldn't find a project at the path {path}")]
    UnknownProjectByPath {
        path: ValidPath,
        #[label = "Couldn't find this project"]
        span: SourceSpan,

        #[source_code]
        source_code: ConfigSource,
    },
    #[diagnostic(help("You can only require tasks from direct or indirect dependencies"))]
    #[error("Tried to require a task from {required_project}, which is not an ancestor of {current_project}")]
    RequiredFromUnrelatedProject {
        required_project: String,
        current_project: String,

        #[label = "You specified {required_project} here"]
        span: SourceSpan,

        #[source_code]
        source_code: ConfigSource,
    },
}

fn resolve_requires(
    requires: config::TaskRequires,
    current_project: &ProjectInfo,
    workspace: &Workspace,
    source: &ConfigSource,
) -> Result<Vec<PossibleTaskRef>, TaskResolutionError> {
    let anchor = match requires.target.anchor.as_ref() {
        TargetAnchor::CurrentProject => current_project,
        TargetAnchor::ProjectByName(name) => workspace.project_by_name(name).ok_or_else(|| {
            TaskResolutionError::UnknownProjectByName {
                name: name.to_string(),
                span: requires.target.anchor.span,
                source_code: source.clone(),
            }
        })?,
        TargetAnchor::ProjectByPath(path) => workspace
            .project_at_path(path.full_path())
            .ok_or_else(|| TaskResolutionError::UnknownProjectByPath {
                path: path.clone(),
                span: requires.target.anchor.span,
                source_code: source.clone(),
            })?,
    };

    if !current_project.has_dependency(&anchor.project_ref(), workspace) {
        return Err(TaskResolutionError::RequiredFromUnrelatedProject {
            required_project: anchor.name.clone(),
            current_project: current_project.name.clone(),
            span: requires.target.anchor.span,
            source_code: source.clone(),
        });
    }

    let projects = match requires.target.selection {
        config::Selection::Project => BTreeSet::from([current_project.project_ref()]),
        config::Selection::ProjectWithDependencies => current_project.dependencies(workspace),
        config::Selection::JustDependencies => {
            let mut deps = current_project.dependencies::<BTreeSet<_>>(workspace);
            deps.remove(&current_project.project_ref());
            deps
        }
    };

    Ok(projects
        .into_iter()
        .map(|project| PossibleTaskRef(project, requires.task.clone()))
        .collect())
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct TaskInputs {
    pub paths: Vec<Glob>,
    pub env_vars: Vec<String>,
    pub commands: Vec<String>,
}

impl TaskInputs {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.env_vars.is_empty() && self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.paths.len() + self.env_vars.len() + self.commands.len()
    }

    pub fn from_config(inputs: &[config::InputBlock]) -> TaskInputs {
        let mut this = TaskInputs::default();
        for input in inputs {
            this.load_block(input)
        }
        this
    }

    fn load_block(&mut self, inputs: &config::InputBlock) {
        for path in &inputs.paths {
            self.paths.push(path.clone().into_inner());
        }

        for _var in &inputs.env_vars {
            self.env_vars.push(_var.to_owned());
            todo!("Haven't implemented env var input support yet");
        }

        for _command in &inputs.commands {
            self.commands.push(_command.to_owned());
            todo!("Haven't implemented command input support yet");
        }
    }
}
