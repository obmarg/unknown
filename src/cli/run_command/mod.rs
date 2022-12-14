use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use camino::{Utf8Path, Utf8PathBuf};
use globset::GlobSet;
use tokio::runtime::Runtime;

use crate::{
    hashing::{HashError, HashRegistry},
    workspace::{ProjectInfo, TaskRef, Workspace},
};

use self::{output::build_command_outputs, runner::TaskRunner};
use super::filters::ProjectFilter;

mod child_ext;
mod output;
mod runner;

#[derive(clap::Parser)]
pub struct RunOpts {
    /// The task or tasks to be run.
    #[clap(value_parser)]
    pub tasks: Vec<String>,

    /// The project or projects to run against.
    ///
    /// If nabs is running in a project this defaults to that project.
    /// Otherwise this defaults to the whole repo.
    #[clap(long)]
    pub filter: Option<super::filters::ProjectFilter>,

    /// An optional git ref to determine what has changed.
    ///
    /// Defaults to changes since the last run of a task (or will once that's implemented)
    #[clap(long)]
    pub since: Option<String>,
}

pub fn run(workspace: Workspace, opts: RunOpts) -> miette::Result<()> {
    let workspace = Arc::new(workspace);

    let target_projects = filter_projects(
        &workspace,
        opts.filter.or_else(|| {
            infer_filter(
                workspace.root_path().as_ref(),
                &workspace.projects_globset(),
            )
        }),
    );
    let tasks = find_tasks(&workspace, &target_projects, opts.tasks);

    tracing::debug!(tasks = ?tasks, "Running tasks");

    if tasks.is_empty() {
        // TODO: think about this output, it's not great.
        println!("No tasks found");
        return Ok(());
    }

    let hash_registry = Arc::new(HashRegistry::for_workspace(&workspace)?);

    // TODO: each task needs a HashSet of TaskRefs for its _direct_ dependencies.

    let rt = Runtime::new().expect("to be able to start tokio runtime");
    rt.block_on(async {
        let tasks = tasks.clone();
        let outputs = build_command_outputs(
            &tasks
                .iter()
                .map(|task| task.task_ref.lookup(&workspace))
                .collect::<Vec<_>>(),
        );
        let mut waiting = tasks
            .iter()
            .map(|task| (task.task_ref.clone(), task.deps.len()))
            .collect::<HashMap<_, _>>();

        let mut ready = Vec::new();
        let mut dependants = HashMap::<_, Vec<_>>::new();
        for task in tasks.into_iter() {
            if task.deps.is_empty() {
                waiting.remove(&task.task_ref);
                ready.push(task.task_ref.clone());
            }
            for dep in task.deps {
                dependants
                    .entry(dep)
                    .or_default()
                    .push(task.task_ref.clone());
            }
        }

        let mut runner = TaskRunner::new(&workspace, opts.since.clone(), outputs, &hash_registry);

        for task in ready.drain(0..).rev() {
            tracing::debug!(%task, "Task has no dependencies, adding to ready list");
            runner.start_task(task);
        }

        while let Some(finished_task) = runner.next_finished().await {
            match finished_task.outcome {
                TaskOutcome::Succesful | TaskOutcome::Skipped => {
                    if !dependants.contains_key(&finished_task.task_ref) {
                        continue;
                    }
                    for dependant in &dependants[&finished_task.task_ref] {
                        let waiting_for = waiting
                            .entry(dependant.clone())
                            .and_modify(|count| *count -= 1)
                            .or_default();
                        if *waiting_for == 0 {
                            waiting.remove(dependant);
                            tracing::debug!(task = %dependant, "All dependencies finished, adding to ready list");
                            runner.start_task(dependant.clone());
                        }
                    }
                }
                TaskOutcome::Failed(_) => {
                    println!("{} failed :(", finished_task.task_ref);
                    // TODO: better reporting.
                    waiting.clear();
                }
            };
        }
    });

    Arc::try_unwrap(hash_registry)
        .map_err(|_| ())
        .expect("to be the exclusive owner of the hash registry")
        .save()
        .expect("to be able to save the TaskRegistry");

    // Now, for each task in tasks:
    // - Check if inputs have changed.
    // - If not, skip.
    // - If yes, run the command.
    //
    // Parallelism can be obtained by having workers that tasks get ferried off to when
    // they're ready.
    //
    // Once each task finishes processing, we trawl the list looking for one thats deps
    // have been satisfied, send the first one off to a worker.  Repeat if there are still
    // free workers.

    Ok(())
}

// Infers the run filter based on the current directory (if any)
fn infer_filter(workspace_root: &Utf8Path, globset: &GlobSet) -> Option<ProjectFilter> {
    let mut current_path =
        Utf8PathBuf::try_from(std::env::current_dir().expect("to have a current directory"))
            .expect("current_dir to be utf8");

    while current_path.starts_with(workspace_root) {
        if current_path.join("project.kdl").exists() {
            let relative_path = current_path.strip_prefix(workspace_root).unwrap();
            if globset.is_match(relative_path) {
                return Some(ProjectFilter::path(relative_path.to_path_buf()));
            }
        }
        if !current_path.pop() {
            break;
        }
    }

    None
}

#[tracing::instrument(skip(workspace))]
fn filter_projects(workspace: &Workspace, filter: Option<ProjectFilter>) -> HashSet<&ProjectInfo> {
    let specs = filter.map(|pf| pf.specs).unwrap_or_default();
    if specs.is_empty() {
        // TODO: If we're being run from within a project automatically filter to
        // that project.
        return workspace.projects().collect();
    }

    let mut cumulative_selection = HashSet::new();
    for spec in specs {
        let mut current_selection = HashSet::new();
        // First determine which projects match this spec.
        for project in workspace.projects() {
            let matches = match &spec.matcher {
                super::filters::Matcher::Path(path) => project.root.as_subpath() == path,
                super::filters::Matcher::Name(name) => project.name == *name,
            };
            if matches {
                current_selection.insert(project);
            }
        }

        // Then pull in any deps from the graph as dictated by the spec.
        if spec.include_dependencies {
            todo!()
        }
        if spec.include_dependents {
            todo!()
        }
        cumulative_selection.extend(current_selection);
    }

    cumulative_selection
}

#[derive(Debug, Clone)]
struct TaskAndDeps {
    task_ref: TaskRef,
    deps: HashSet<TaskRef>,
}

#[tracing::instrument(
    fields(
        target_projects = %target_projects.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>().join(", ")
    )
    skip(workspace, target_projects))
]
fn find_tasks<'a>(
    workspace: &'a Workspace,
    target_projects: &HashSet<&'a ProjectInfo>,
    task_list: Vec<String>,
) -> Vec<TaskAndDeps> {
    let graph = workspace.graph();
    let mut tasks = HashSet::new();
    for project in target_projects {
        for task_name in &task_list {
            if let Some(task) = project.lookup_task(task_name, workspace) {
                tasks.insert(task.task_ref());
                let task_deps = graph.walk_task_dependencies(task.task_ref());
                tracing::debug!(
                    "{} depends on {}",
                    task.task_ref(),
                    task_deps
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                tasks.extend(task_deps);
            }
        }
    }
    // Now we return them in topsorted order with their set of
    // direct deps.
    graph
        .topsort_tasks()
        .into_iter()
        .filter(|task_ref| tasks.contains(task_ref))
        .map(|task_ref| TaskAndDeps {
            deps: task_ref.direct_dependencies(workspace),
            task_ref,
        })
        .collect()
}

#[derive(thiserror::Error, Debug)]
enum TaskError {
    #[error("Error running git: {0}")]
    GitError(#[from] crate::git::GitError),
    #[error("Error running task command: {0}")]
    CommandError(std::io::Error),
    #[error("Error hashing inputs or outputs: {0}")]
    Hashing(#[from] HashError),
    // #[error("Uncountered a path that wasn't UTF8: {0}")]
    // InvalidPathFound(#[from] camino::FromPathBufError),
    #[error("Error reading command output")]
    // TODO: Add fields to this.
    OutputError(),
}

struct FinishedTask {
    task_ref: TaskRef,
    outcome: TaskOutcome,
}

enum TaskOutcome {
    Skipped,
    Succesful,
    Failed(TaskError),
}
