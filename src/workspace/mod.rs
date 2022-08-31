use std::collections::{HashMap, HashSet};

use crate::config;

use self::graph::WorkspaceGraph;

mod graph;

struct Workspace {
    info: WorkspaceInfo,
    project_map: HashMap<String, ProjectInfo>,
    graph: graph::WorkspaceGraph,
}

struct WorkspaceInfo {
    name: String,
    project_paths: Vec<String>,
}

impl Workspace {
    fn new(workspace_file: config::WorkspaceFile, project_files: Vec<config::ProjectFile>) -> Self {
        let workspace_info = WorkspaceInfo {
            name: workspace_file.name,
            project_paths: workspace_file.project_paths,
        };

        let project_names = project_files
            .iter()
            .map(|project_file| &project_file.project)
            .collect::<HashSet<_>>();

        let mut project_map = HashMap::with_capacity(project_files.len());

        for project_file in &project_files {
            let mut dependencies = Vec::new();
            // TODO: handle other dependencies
            for project in &project_file.dependencies.projects {
                if !project_names.contains(&project) {
                    panic!("Unknown project: {project}");
                }
                dependencies.push(ProjectRef(project.clone()));
            }

            project_map.insert(
                project_file.project.clone(),
                ProjectInfo {
                    name: project_file.project.clone(),
                    dependencies,
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

struct ProjectInfo {
    name: String,
    dependencies: Vec<ProjectRef>,
    // TODO:
}

pub struct ProjectRef(String);
