use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::config;

use self::graph::WorkspaceGraph;

mod graph;

pub struct Workspace {
    info: WorkspaceInfo,
    project_map: HashMap<String, ProjectInfo>,
    graph: graph::WorkspaceGraph,
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
}

impl Workspace {
    pub fn new(
        workspace_file: config::WorkspaceFile,
        project_files: Vec<config::ProjectFile>,
    ) -> Self {
        let workspace_info = WorkspaceInfo {
            name: workspace_file.name,
            project_paths: workspace_file.project_paths,
        };

        let project_names = project_files
            .iter()
            .map(|project_file| &project_file.config.project)
            .collect::<HashSet<_>>();

        let mut project_map = HashMap::with_capacity(project_files.len());

        for project_file in &project_files {
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
                    root: project_file.project_root.clone(),
                },
            );
        }

        Workspace {
            graph: WorkspaceGraph::new(&workspace_info, &project_map),
            info: workspace_info,
            project_map,
        }
    }
}

#[derive(Debug)]
struct ProjectInfo {
    name: String,
    dependencies: Vec<ProjectRef>,
    tasks: Vec<TaskInfo>,
    root: PathBuf,
}

#[derive(Debug)]
pub struct ProjectRef(String);

// TODO: Think about sticking this in an arc or similar rather than clone
#[derive(Clone, Debug)]
pub struct TaskInfo {
    name: String,
    commands: Vec<String>,
    dependencies: Vec<TaskDependency>,
}

#[derive(Clone, Debug)]
struct TaskDependency {
    task: TaskDependencySpec,
    target: TaskDependencyTarget,
}

#[derive(Clone, Debug)]
enum TaskDependencySpec {
    NamedTask(String),

    // TODO: Implement the TaggedTask support
    TaggedTask,
}

#[derive(Clone, Debug)]
enum TaskDependencyTarget {
    CurrentProject,
    DependencyProjects,
    DependencyProjectsAndCurrent,
}

impl TaskDependency {
    fn from_config(config: &config::TaskDependency) -> Self {
        TaskDependency {
            task: TaskDependencySpec::NamedTask(config.task.clone()),
            target: TaskDependencyTarget::CurrentProject,
        }
    }
}
