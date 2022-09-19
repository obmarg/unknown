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
};
use super::filters::ProjectFilter;

mod child_ext;
mod output;

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
        let mut outputs = build_command_outputs(
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

        let mut currently_running = FuturesUnordered::new();
        for task in ready.drain(0..).rev() {
            let workspace = Arc::clone(&workspace);
            let since = opts.since.clone();
            let hash_registry = Arc::clone(&hash_registry);

            let output = outputs
                .remove(&task)
                .expect("a CommandOutput to exist for every task");

            currently_running.push(tokio::spawn(async move {
                // TODO: ok, so basically:
                // Prior to running a task we want to hash its inputs regardless
                // of whether we're actually going to use that hash.
                // Then when task is succesful we update the HashRegistry with that hash.
                // If task is not succesful, do not update.  Simples.
                //
                // Also need to think about how changes in project deps interact with this...
                //
                // So I guess at this point we need to know what the inputs
                // of a task even are.
                //
                // By default it's all (non ignored) files in the project.
                let result = run_task(
                    task.lookup(&workspace),
                    &workspace,
                    output,
                    since,
                    &hash_registry,
                )
                .await
                .map(|_| task.clone());
                result.map_err(|_| task)
            }));
        }

        while let Some(join_result) = currently_running.next().await {
            match join_result {
                Ok(Ok(successful_task)) => {
                    if !dependants.contains_key(&successful_task) {
                        continue;
                    }
                    for dependant in &dependants[&successful_task] {
                        let waiting_for = waiting
                            .entry(dependant.clone())
                            .and_modify(|count| *count -= 1)
                            .or_default();
                        if *waiting_for == 0 {
                            waiting.remove(dependant);

                            let workspace = Arc::clone(&workspace);
                            let task = dependant.clone();
                            let since = opts.since.clone();
                            let hash_registry = Arc::clone(&hash_registry);

                            let output = outputs
                                .remove(&task)
                                .expect("a CommandOutput to exist for every task");

                            currently_running.push(tokio::spawn(async move {
                                let result = run_task(
                                    task.lookup(&workspace),
                                    &workspace,
                                    output,
                                    since,
                                    &hash_registry,
                                )
                                .await
                                .map(|_| task.clone());
                                result.map_err(|_| task)
                            }));
                        }
                    }
                }
                Ok(Err(failed_task)) => {
                    // TODO: report the failure somehow.
                    waiting.clear();
                }
                Err(join_err) => {
                    todo!()
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

async fn run_task(
    task: &TaskInfo,
    workspace: &Workspace,
    output: CommandOutput,
    since: Option<String>,
    hash_registry: &HashRegistry,
) -> Result<(), TaskError> {
    let (should_run, input_hash) =
        block_in_place(|| should_task_run(task, workspace, since, hash_registry))?;

    if !should_run {
        return Ok(());
    }

    let mut output = output;

    for command in &task.commands {
        let mut args = command.split(' ');
        let command = args
            .next()
            .expect("there to be some content in a tasks command");

        let mut child = tokio::process::Command::new(command)
            .args(args)
            .current_dir(&task.project.lookup(workspace).root)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(TaskError::CommandError)?;

        child
            .wait_and_pipe_output(&mut output)
            .await
            .map_err(|_| TaskError::OuptutError())?;
        // TODO: Do something with the result of this...
    }

    if let Some(input_hash) = input_hash {
        hash_registry.update_input_hash(task.task_ref(), input_hash);
    }

    Ok(())
}

fn should_task_run(
    task: &TaskInfo,
    workspace: &Workspace,
    since: Option<String>,
    hash_registry: &HashRegistry,
) -> Result<(bool, Option<Hash>), TaskError> {
    let project = task.project.lookup(workspace);

    match since {
        Some(since) => {
            let project_root = project.root.clone();
            let should_run = git::have_files_changed(since, project_root.into())?;

            Ok((should_run, None))
        }
        None => {
            let new_hash = hash_task_inputs(project, task)?;
            let last_hash = hash_registry
                .lookup(&task.task_ref())
                .and_then(|h| h.inputs);

            let should_run = last_hash
                .as_ref()
                .map(|last_hash| *last_hash != new_hash)
                .unwrap_or(true);

            Ok((should_run, Some(new_hash)))
        }
    }
}
