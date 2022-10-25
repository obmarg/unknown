use std::collections::{HashMap, HashSet};

use petgraph::{
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, EdgeFiltered, IntoNeighbors, Walker},
};

use super::{ProjectInfo, ProjectRef, TaskDependencySpec, TaskRef, WorkspaceInfo};

type Graph = petgraph::Graph<WorkspaceNode, WorkspaceEdge>;

pub struct WorkspaceGraph {
    graph: Graph,
    project_indices: HashMap<String, NodeIndex>,
    task_indices: HashMap<TaskRef, NodeIndex>,
}

impl WorkspaceGraph {
    pub(super) fn new(info: &WorkspaceInfo, project_map: &HashMap<String, ProjectInfo>) -> Self {
        let mut graph = Graph::new();
        let root_index = graph.add_node(WorkspaceNode::WorkspaceRoot);

        let mut project_indices = HashMap::new();
        for name in project_map.keys() {
            let node_index = graph.add_node(WorkspaceNode::Project(ProjectRef(name.clone())));
            graph.add_edge(root_index, node_index, WorkspaceEdge::HasProject);

            project_indices.insert(name.clone(), node_index);
        }

        let mut task_indices = HashMap::new();

        for project_info in project_map.values() {
            // Create tasks & their corresponding edges
            for task in &project_info.tasks {
                let task_index = graph.add_node(WorkspaceNode::Task(task.task_ref()));
                task_indices.insert(task.task_ref(), task_index);
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
                    WorkspaceEdge::ProjectDependsOn,
                );
                graph.add_edge(
                    project_indices[&dependency.0],
                    project_indices[&project_info.name],
                    WorkspaceEdge::ProjectDependedOnBy,
                );
            }
        }

        // TODO: Validate the graph to make sure there's no cycles?

        let mut rv = WorkspaceGraph {
            graph,
            project_indices,
            task_indices,
        };

        rv.generate_task_edges(project_map);

        rv
    }

    fn generate_task_edges(&mut self, project_map: &HashMap<String, ProjectInfo>) {
        for project in project_map.values() {
            for task in &project.tasks {
                let current_task_index = self.task_indices[&task.task_ref()];

                for dependency in &task.dependencies {
                    let TaskDependencySpec::NamedTask(dependency_name) = &dependency.task;
                    if dependency.target_self {
                        let maybe_index = project
                            .lookup_task(dependency_name)
                            .and_then(|task| self.task_indices.get(&task.task_ref()));

                        if let Some(target_index) = maybe_index {
                            if current_task_index != *target_index {
                                self.graph.add_edge(
                                    current_task_index,
                                    *target_index,
                                    WorkspaceEdge::TaskDependsOn,
                                );
                                self.graph.add_edge(
                                    *target_index,
                                    current_task_index,
                                    WorkspaceEdge::TaskDependedOnBy,
                                );
                            }
                        }
                    }

                    if dependency.target_deps {
                        for project_dep in &project.dependencies {
                            let project_dep = &project_map[&project_dep.0];

                            let maybe_index = project_dep
                                .lookup_task(dependency_name)
                                .and_then(|task| self.task_indices.get(&task.task_ref()));

                            if let Some(target_index) = maybe_index {
                                self.graph.add_edge(
                                    current_task_index,
                                    *target_index,
                                    WorkspaceEdge::TaskDependsOn,
                                );
                                self.graph.add_edge(
                                    *target_index,
                                    current_task_index,
                                    WorkspaceEdge::TaskDependedOnBy,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn dot(&self) -> petgraph::dot::Dot<'_, &Graph> {
        petgraph::dot::Dot::new(&self.graph)
    }

    pub fn walk_project_dependents(&self, project: ProjectRef) -> HashSet<ProjectRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::ProjectDependedOnBy)
        });

        DfsPostOrder::new(&filtered_graph, self.project_indices[&project.0])
            .iter(&filtered_graph)
            .filter_map(|index| match &self.graph[index] {
                WorkspaceNode::Project(project_ref) => Some(project_ref.clone()),
                WorkspaceNode::Task(_) | WorkspaceNode::WorkspaceRoot => None,
            })
            .collect()
    }

    pub fn walk_project_dependencies(&self, project: ProjectRef) -> HashSet<ProjectRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::ProjectDependsOn)
        });

        DfsPostOrder::new(&filtered_graph, self.project_indices[&project.0])
            .iter(&filtered_graph)
            .filter_map(|index| match &self.graph[index] {
                WorkspaceNode::Project(project_ref) => Some(project_ref.clone()),
                WorkspaceNode::Task(_) | WorkspaceNode::WorkspaceRoot => None,
            })
            .collect()
    }

    pub fn walk_task_dependencies(&self, task: TaskRef) -> HashSet<TaskRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::TaskDependsOn)
        });

        DfsPostOrder::new(&filtered_graph, self.task_indices[&task])
            .iter(&filtered_graph)
            .filter_map(|index| match &self.graph[index] {
                WorkspaceNode::Task(task_ref) => Some(task_ref.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn topsort_tasks(&self) -> Vec<TaskRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(
                edge.weight(),
                WorkspaceEdge::HasProject | WorkspaceEdge::HasTask | WorkspaceEdge::TaskDependsOn
            )
        });

        petgraph::algo::toposort(&filtered_graph, None)
            .expect("Workspace graph shouldn't have cycles")
            .into_iter()
            .filter_map(|index| match &self.graph[index] {
                WorkspaceNode::Task(task_ref) => Some(task_ref.clone()),
                WorkspaceNode::WorkspaceRoot | WorkspaceNode::Project(_) => None,
            })
            .collect()
    }

    fn lookup_task(&self, index: NodeIndex) -> Option<TaskRef> {
        match &self.graph[index] {
            WorkspaceNode::Task(task_ref) => Some(task_ref.clone()),
            _ => None,
        }
    }
}

impl TaskRef {
    pub fn direct_dependencies(&self, workspace: &super::Workspace) -> HashSet<TaskRef> {
        let workspace = &workspace.graph;

        let filtered_graph = EdgeFiltered::from_fn(&workspace.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::TaskDependsOn)
        });

        filtered_graph
            .neighbors(workspace.task_indices[self])
            .filter_map(|index| workspace.lookup_task(index))
            .collect()
    }
}

#[derive(Debug)]
pub enum WorkspaceNode {
    WorkspaceRoot,
    Project(ProjectRef),
    Task(TaskRef),
}

#[derive(Debug)]
pub enum WorkspaceEdge {
    HasProject,
    ProjectDependsOn,
    ProjectDependedOnBy,
    TaskDependsOn,
    TaskDependedOnBy,
    HasTask,
}

// TODO: Some tests of this graph stuff would probably not go amiss (or
// more thorough workspace tests...)
