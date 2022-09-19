use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    process::Stdio,
    sync::Arc,
};

use camino::Utf8PathBuf;
use futures::{stream::FuturesUnordered, StreamExt};
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use tabled::{Table, Tabled};
use tokio::{
    io::AsyncReadExt,
    runtime::Runtime,
    task::{block_in_place, spawn_blocking},
};

use crate::{
    git,
    hashing::{hash_task_inputs, Hash, HashError, HashRegistry},
    workspace::{
        self, ProjectInfo, ProjectRef, TaskDependencySpec, TaskInfo, TaskRef, Workspace,
        WorkspacePath,
    },
};

use self::{
    child_ext::ChildExt,
    output::{build_command_outputs, CommandOutput},
    runner::TaskRunner,
};
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

    let target_projects = filter_projects(&workspace, opts.filter);
    let tasks = find_tasks(&workspace, &target_projects, opts.tasks);

    if tasks.is_empty() {
        // TODO: log something maybe?
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
                            runner.start_task(dependant.clone());
                        }
                    }
                }
                TaskOutcome::Failed(_) => {
                    // TODO: report the failure somehow.
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
                super::filters::Matcher::Path(_) => todo!(),
                super::filters::Matcher::Name(name) => project.name == *name,
                super::filters::Matcher::Exclude(_) => todo!(),
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

fn find_tasks<'a>(
    workspace: &'a Workspace,
    target_projects: &HashSet<&'a ProjectInfo>,
    task_list: Vec<String>,
) -> Vec<TaskAndDeps> {
    let mut tasks = HashSet::new();
    for project in target_projects {
        for task_name in &task_list {
            if let Some(task) = project.lookup_task(task_name) {
                tasks.insert(task.task_ref());
                let task_deps = workspace.graph.walk_task_dependencies(task.task_ref());
                tasks.extend(task_deps);
            }
        }
    }

    // Now we return them in topsorted order with their set of
    // direct deps.
    workspace
        .graph
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
    OuptutError(),
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
