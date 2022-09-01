use std::collections::HashMap;

use petgraph::Graph;

use super::{ProjectInfo, ProjectRef, Workspace, WorkspaceInfo};

pub struct WorkspaceGraph {
    graph: Graph<WorkspaceNode, WorkspaceEdge>,
}

impl WorkspaceGraph {
    pub(super) fn new(info: &WorkspaceInfo, project_map: &HashMap<String, ProjectInfo>) -> Self {
        let mut project_indices = HashMap::new();
        let mut graph = Graph::new();
        for name in project_map.keys() {
            let node_index = graph.add_node(WorkspaceNode::Project(ProjectRef(name.clone())));
            project_indices.insert(name, node_index);
        }

        for project_info in project_map.values() {
            // Create tasks & their corresponding edges
            for task in &project_info.tasks {
                let task_index = graph.add_node(WorkspaceNode::Task(
                    ProjectRef(project_info.name.clone()),
                    task.clone(),
                ));
                graph.add_edge(
                    project_indices[&project_info.name],
                    task_index,
                    WorkspaceEdge::HasTask,
                );
            }

            // Create project dependency edges
            for dependency in &project_info.dependencies {
                graph.add_edge(
                    project_indices[&project_info.name],
                    project_indices[&dependency.0],
                    WorkspaceEdge::DependsOn,
                );
            }
        }

        // TODO: Validate the graph to make sure there's no cycles?

        WorkspaceGraph { graph }
    }
}

pub enum WorkspaceNode {
    Project(ProjectRef),
    Task(ProjectRef, super::TaskInfo),
}

pub enum WorkspaceEdge {
    DependsOn,
    HasTask,
}
