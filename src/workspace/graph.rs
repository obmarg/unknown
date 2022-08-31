use std::collections::HashMap;

use petgraph::Graph;

use super::{ProjectInfo, ProjectRef, Workspace, WorkspaceInfo};

pub struct WorkspaceGraph {
    graph: Graph<WorkspaceNode, WorkspaceEdge>,
}

impl WorkspaceGraph {
    pub(super) fn new(info: &WorkspaceInfo, project_map: &HashMap<String, ProjectInfo>) -> Self {
        let mut node_indices = HashMap::new();
        let mut graph = Graph::new();
        for name in project_map.keys() {
            let node_index = graph.add_node(WorkspaceNode::Project(ProjectRef(name.clone())));
            node_indices.insert(name, node_index);
        }

        for project_info in project_map.values() {
            for dependency in &project_info.dependencies {
                graph.add_edge(
                    node_indices[&project_info.name],
                    node_indices[&dependency.0],
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
}

pub enum WorkspaceEdge {
    DependsOn,
}
