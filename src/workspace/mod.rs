use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::config;

use self::graph::WorkspaceGraph;

mod graph;
mod paths;

pub use paths::WorkspacePath;

pub struct Workspace {
    info: WorkspaceInfo,
    project_map: HashMap<String, ProjectInfo>,
    pub graph: graph::WorkspaceGraph,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("info", &self.info)
            .field("project_map", &self.project_map)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct WorkspaceInfo {
    name: String,
    project_paths: Vec<String>,
    root_path: WorkspacePath,
}

impl Workspace {
    pub fn new(
        workspace_file: config::WorkspaceFile,
        project_files: Vec<config::ProjectFile>,
    ) -> Self {
        let workspace_info = WorkspaceInfo {
            name: workspace_file.config.name,
            project_paths: workspace_file.config.project_paths,
            root_path: WorkspacePath::for_workspace(&workspace_file.workspace_root),
        };

        let project_names = project_files
            .iter()
            .map(|project_file| &project_file.config.project)
            .collect::<HashSet<_>>();

        let mut project_map = HashMap::with_capacity(project_files.len());

        for project_file in &project_files {
            let project_ref = ProjectRef(project_file.config.project.clone());

            let mut dependencies = Vec::new();
            // TODO: handle other dependencies
            for project in &project_file.config.dependencies.projects {
                if !project_names.contains(&project) {
                    panic!("Unknown project: {project}");
                }
                dependencies.push(ProjectRef(project.clone()));
            }

            let mut tasks = Vec::new();
            // TODO: handle task imports
            for task in &project_file.config.tasks.tasks {
                tasks.push(TaskInfo {
                    project: project_ref.clone(),
                    name: task.name.clone(),
                    commands: task.commands.clone(),
                    dependencies: task
                        .dependencies
                        .iter()
                        .map(TaskDependency::from_config)
                        .collect(),
                })
            }

            project_map.insert(
                project_file.config.project.clone(),
                ProjectInfo {
                    name: project_file.config.project.clone(),
                    dependencies,
                    tasks,
                    root: workspace_info.root_path.subpath(&project_file.project_root),
                },
            );
        }

        Workspace {
            graph: WorkspaceGraph::new(&workspace_info, &project_map),
            info: workspace_info,
            project_map,
        }
    }

    pub fn projects(&self) -> impl Iterator<Item = &ProjectInfo> {
        self.project_map.values()
    }

    pub fn lookup(&self, project_ref: &ProjectRef) -> &ProjectInfo {
        // TODO: This basically assumes a ProjectRef is always valid.
        // Probably need to enforce that with types somehow or make this return an option
        &self.project_map[&project_ref.0]
    }
}

impl TaskRef {
    pub fn lookup<'a>(&self, workspace: &'a Workspace) -> &'a TaskInfo {
        workspace.project_map[&self.0 .0]
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
    pub root: WorkspacePath,
}

impl ProjectInfo {
    pub fn project_ref(&self) -> ProjectRef {
        ProjectRef(self.name.clone())
    }

    pub fn lookup_task(&self, name: &str) -> Option<&TaskInfo> {
        self.tasks.iter().find(|task| task.name == name)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ProjectRef(String);

impl ProjectRef {
    pub fn name(&self) -> &str {
        return &self.0;
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TaskRef(ProjectRef, String);

// TODO: Think about sticking this in an arc or similar rather than clone
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TaskInfo {
    pub project: ProjectRef,
    pub name: String,
    pub commands: Vec<String>,
    pub dependencies: Vec<TaskDependency>,
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
