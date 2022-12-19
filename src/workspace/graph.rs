use std::collections::{HashMap, HashSet};

use petgraph::{
    algo::has_path_connecting,
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, EdgeFiltered, IntoNeighbors, Walker},
};

use super::{PossibleTaskRef, ProjectInfo, ProjectRef, TaskInfo, TaskRef};

type Graph = petgraph::Graph<WorkspaceNode, WorkspaceEdge>;

pub struct WorkspaceGraph {
    graph: Graph,
    project_indices: HashMap<ProjectRef, NodeIndex>,
    task_indices: HashMap<TaskRef, NodeIndex>,
    root_index: NodeIndex,
}

impl WorkspaceGraph {
    pub(super) fn new() -> Self {
        let mut graph = Graph::new();
        let root_index = graph.add_node(WorkspaceNode::WorkspaceRoot);

        WorkspaceGraph {
            graph,
            project_indices: HashMap::new(),
            task_indices: HashMap::new(),
            root_index,
        }
    }

    pub(super) fn add_projects(&mut self, project_map: &HashMap<ProjectRef, ProjectInfo>) {
        let graph = &mut self.graph;

        for (name, project) in project_map.iter() {
            let node_index = graph.add_node(WorkspaceNode::Project(project.project_ref()));
            graph.add_edge(self.root_index, node_index, WorkspaceEdge::HasProject);

            self.project_indices.insert(name.clone(), node_index);
        }

        for project_info in project_map.values() {
            // Create project dependency edges
            for dependency in &project_info.dependencies {
                graph.add_edge(
                    self.project_indices[&project_info.project_ref()],
                    self.project_indices[dependency],
                    WorkspaceEdge::ProjectDependsOn,
                );
                graph.add_edge(
                    self.project_indices[dependency],
                    self.project_indices[&project_info.project_ref()],
                    WorkspaceEdge::ProjectDependedOnBy,
                );
            }
        }

        // TODO: Validate the graph to make sure there's no cycles?
    }

    pub(super) fn register_tasks(&mut self, task_map: &HashMap<TaskRef, TaskInfo>) {
        // Create tasks & their corresponding edges
        for task in task_map.values() {
            let task_index = self.graph.add_node(WorkspaceNode::Task(task.task_ref()));
            self.task_indices.insert(task.task_ref(), task_index);
            self.graph.add_edge(
                self.project_indices[task.task_ref().project()],
                task_index,
                WorkspaceEdge::HasTask,
            );
        }
        self.generate_task_edges(task_map);
    }

    fn generate_task_edges(&mut self, task_map: &HashMap<TaskRef, TaskInfo>) {
        // Also quite hard here.  Easy to look up self or project by path.  Difficult by name.
        // Could simplify by transforming name into path at an earlier stage possibly?
        for task in task_map.values() {
            let current_task_index = self.task_indices[&task.task_ref()];

            for PossibleTaskRef(project_ref, task) in &task.requires {
                let actual_ref = TaskRef(project_ref.clone(), task.clone());
                if task_map.contains_key(&actual_ref) {
                    // The required task exists, so hook it up.
                    let target_index = self.task_indices[&actual_ref];
                    if current_task_index != target_index {
                        self.graph.add_edge(
                            current_task_index,
                            target_index,
                            WorkspaceEdge::TaskDependsOn,
                        );
                        self.graph.add_edge(
                            target_index,
                            current_task_index,
                            WorkspaceEdge::TaskDependedOnBy,
                        );
                    }
                } else {
                    // TODO: Do we want to error/warn here?
                    // Currently we just ignore unmet requirements but not sure if that's a good thing....
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

        DfsPostOrder::new(&filtered_graph, self.project_indices[&project])
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

    pub fn project_tasks(&self, project_ref: &ProjectRef) -> Vec<TaskRef> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::HasTask)
        });

        filtered_graph
            .neighbors(self.project_indices[project_ref])
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
        let graph = workspace.graph();

        let filtered_graph = EdgeFiltered::from_fn(&graph.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::TaskDependsOn)
        });

        filtered_graph
            .neighbors(graph.task_indices[self])
            .filter_map(|index| graph.lookup_task(index))
            .collect()
    }
}

impl ProjectInfo {
    pub fn has_dependency(&self, other_project: &ProjectRef, workspace: &super::Workspace) -> bool {
        let graph = workspace.graph();

        let filtered_graph = EdgeFiltered::from_fn(&graph.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::ProjectDependsOn)
        });

        has_path_connecting(
            &filtered_graph,
            graph.project_indices[&self.project_ref()],
            graph.project_indices[other_project],
            None,
        )
    }

    pub fn dependencies<B>(&self, workspace: &super::Workspace) -> B
    where
        B: FromIterator<ProjectRef>,
    {
        let graph = workspace.graph();

        let filtered_graph = EdgeFiltered::from_fn(&graph.graph, |edge| {
            matches!(edge.weight(), WorkspaceEdge::ProjectDependsOn)
        });

        DfsPostOrder::new(&filtered_graph, graph.project_indices[&self.project_ref()])
            .iter(&filtered_graph)
            .filter_map(|index| match &graph.graph[index] {
                WorkspaceNode::Project(project_ref) => Some(project_ref.clone()),
                WorkspaceNode::Task(_) | WorkspaceNode::WorkspaceRoot => None,
            })
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
