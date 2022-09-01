use std::collections::{HashMap, HashSet};

use petgraph::{
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, EdgeFiltered, Walker},
};

use super::{ProjectInfo, ProjectRef, Workspace, WorkspaceInfo};

type Graph = petgraph::Graph<WorkspaceNode, WorkspaceEdge>;

pub struct WorkspaceGraph {
    graph: Graph,
    project_indices: HashMap<String, NodeIndex>,
}

impl WorkspaceGraph {
    pub(super) fn new(info: &WorkspaceInfo, project_map: &HashMap<String, ProjectInfo>) -> Self {
        let mut project_indices = HashMap::new();
        let mut graph = Graph::new();
        for name in project_map.keys() {
            let node_index = graph.add_node(WorkspaceNode::Project(ProjectRef(name.clone())));
            project_indices.insert(name.clone(), node_index);
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
                graph.add_edge(
                    project_indices[&dependency.0],
                    project_indices[&project_info.name],
                    WorkspaceEdge::DependedOnBy,
                );
            }
        }

        // TODO: Validate the graph to make sure there's no cycles?

        WorkspaceGraph {
            graph,
            project_indices,
        }
    }

    pub fn dot(&self) -> petgraph::dot::Dot<'_, &Graph> {
        petgraph::dot::Dot::new(&self.graph)
    }

    pub fn walk_project_dependencies(&self, project: ProjectRef) -> HashSet<ProjectRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::DependedOnBy)
        });

        DfsPostOrder::new(&filtered_graph, self.project_indices[&project.0])
            .iter(&filtered_graph)
            .filter_map(|index| match &self.graph[index] {
                WorkspaceNode::Project(project_ref) => Some(project_ref.clone()),
                WorkspaceNode::Task(_, _) => None,
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum WorkspaceNode {
    Project(ProjectRef),
    Task(ProjectRef, super::TaskInfo),
}

#[derive(Debug)]
pub enum WorkspaceEdge {
    DependsOn,
    DependedOnBy,
    HasTask,
}
