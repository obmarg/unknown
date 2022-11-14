use std::collections::{HashMap, HashSet};

use once_cell::sync::OnceCell;

use crate::config::{self, ValidPath, WorkspaceRoot};

use self::graph::WorkspaceGraph;

mod graph;

#[cfg(test)]
mod tests;

use camino::Utf8Path;
use globset::Glob;

pub struct Workspace {
    pub info: WorkspaceInfo,
    graph_: OnceCell<WorkspaceGraph>,
    project_map: HashMap<ProjectRef, ProjectInfo>,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert project_map to a BTreeMap before printing so
        // we can get a consistent order for snapshot testing
        let project_map = self
            .project_map
            .iter()
            .collect::<std::collections::BTreeMap<_, _>>();

        f.debug_struct("Workspace")
            .field("info", &self.info)
            .field("project_map", &project_map)
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
    pub fn new(
        workspace_file: config::WorkspaceFile,
        project_files: Vec<config::ValidProjectFile>,
    ) -> Self {
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

        let project_paths = project_files
            .iter()
            .map(|project_file| project_file.project_root.clone())
            .collect::<HashSet<_>>();

        let mut project_map = HashMap::with_capacity(project_files.len());

        for project_file in project_files {
            let project_ref = ProjectRef(project_file.project_root.clone());

            let mut dependencies = Vec::new();
            for path in project_file.config.dependencies.projects {
                let path = path
                    .into_normalised()
                    .expect("the parser to have normalised ConfigPaths");

                if !project_paths.contains(&path) {
                    panic!("Unknown project: {path}");
                }
                dependencies.push(ProjectRef(path));
            }

            let mut tasks = Vec::new();
            // TODO: handle task imports
            for task in &project_file.config.tasks.tasks {
                tasks.push(TaskInfo {
                    project_name: project_file.config.project.clone(),
                    project: project_ref.clone(),
                    name: task.name.clone(),
                    commands: task.commands.clone(),
                    dependencies: task
                        .dependencies
                        .iter()
                        .map(TaskDependency::from_config)
                        .collect(),
                    inputs: TaskInputs::from_config(&task.input_blocks),
                })
            }

            project_map.insert(
                project_ref.clone(),
                ProjectInfo {
                    name: project_file.config.project.clone(),
                    dependencies,
                    tasks,
                    root: project_file.project_root,
                },
            );
        }

        Workspace {
            graph_: OnceCell::new(),
            info: workspace_info,
            project_map,
        }
    }

    pub fn graph(&self) -> &WorkspaceGraph {
        self.graph_
            .get_or_init(|| WorkspaceGraph::new(&self.info, &self.project_map))
    }

    pub fn projects(&self) -> impl Iterator<Item = &ProjectInfo> {
        self.project_map.values()
    }

    pub fn project_at_path(&self, path: impl AsRef<Utf8Path>) -> Option<&ProjectInfo> {
        // TODO: Ok, so this one definitely wants to be
        let project_ref = ProjectRef(self.info.root_path.normalise_absolute(path.as_ref()).ok()?);
        self.project_map.get(&project_ref)
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
        workspace.project_map[&self.0]
            .tasks
            .iter()
            .find(|task| task.name == self.1)
            .expect("a valid TaskRef for the given Workspace")
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub dependencies: Vec<ProjectRef>,
    pub tasks: Vec<TaskInfo>,
    pub root: ValidPath,
}

impl ProjectInfo {
    pub fn project_ref(&self) -> ProjectRef {
        ProjectRef(self.root.clone())
    }

    pub fn lookup_task(&self, name: &str) -> Option<&TaskInfo> {
        self.tasks.iter().find(|task| task.name == name)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectRef(ValidPath);

impl ProjectRef {
    pub fn as_str(&self) -> &str {
        self.0.as_subpath().as_str()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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

// TODO: Think about sticking this in an arc or similar rather than clone
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TaskInfo {
    pub project: ProjectRef,
    pub project_name: String,
    pub name: String,
    pub commands: Vec<String>,
    pub dependencies: Vec<TaskDependency>,
    pub inputs: TaskInputs,
}

impl TaskInfo {
    pub fn task_ref(&self) -> TaskRef {
        TaskRef(self.project.clone(), self.name.clone())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TaskDependency {
    pub task: TaskDependencySpec,
    pub target_self: bool,
    pub target_deps: bool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum TaskDependencySpec {
    NamedTask(String),
    // TODO: Implement the TaggedTask support
    // TaggedTask,
}

impl TaskDependency {
    fn from_config(config: &config::TaskDependency) -> Self {
        TaskDependency {
            task: TaskDependencySpec::NamedTask(config.task.clone()),
            target_self: config.include_this_package.unwrap_or(true),
            target_deps: config.for_project_deps.unwrap_or_default(),
        }
    }
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
