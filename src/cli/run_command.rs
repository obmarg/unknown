use std::{
    collections::{HashMap, HashSet},
    io::Read,
    process::Stdio,
    sync::Arc,
};

use tabled::{Table, Tabled};
use tokio::runtime::Runtime;

use crate::{
    git,
    workspace::{
        self, ProjectInfo, ProjectRef, TaskDependencySpec, TaskInfo, TaskRef, Workspace,
        WorkspacePath,
    },
};

use super::filters::ProjectFilter;

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

    // TODO: each task needs a HashSet of TaskRefs for its _direct_ dependencies.

    let (task_ready, tasks_ready) = crossbeam::channel::unbounded::<TaskRef>();
    let (task_finished, tasks_finished) =
        crossbeam::channel::unbounded::<Result<TaskRef, TaskRef>>();

    let rt = Runtime::new().expect("to be able to start tokio runtime");
    rt.block_on(async {
        let tasks = tasks.clone();
        // The supervisor process
        let supervisor = tokio::spawn(async move {
            let tasks = tasks.clone();
            let mut waiting = tasks
                .iter()
                .map(|task| (task.task_ref.clone(), task.deps.len()))
                .collect::<HashMap<_, _>>();

            let mut to_start = Vec::new();
            let mut dependants = HashMap::<_, Vec<_>>::new();
            for task in tasks.into_iter() {
                if task.deps.is_empty() {
                    waiting.remove(&task.task_ref);
                    to_start.push(task.task_ref.clone());
                }
                for dep in task.deps {
                    dependants
                        .entry(dep)
                        .or_default()
                        .push(task.task_ref.clone());
                }
            }

            // Any tasks without deps can go into the work pool first.
            for task in to_start.into_iter().rev() {
                task_ready.send(task).unwrap();
            }

            let mut tasks_done = HashSet::<TaskRef>::new();
            while !waiting.is_empty() {
                match tasks_finished.recv().unwrap() {
                    Ok(successful_task) => {
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
                                task_ready.send(dependant.clone()).unwrap();
                            }
                        }
                    }
                    Err(failed_task) => {
                        // TODO: report the failure somehow.
                        waiting.clear();
                    }
                }

                // Wait for tasks to finish.
                // When they do, decrement waiting.
                // If the thing being decremented has no deps then execute it and pop it.
            }
            drop(task_ready);
        });

        // Spawn some worker processes
        // TODO: Pick better numbers
        let mut workers = Vec::new();
        for _ in 0..5 {
            let task_finished = task_finished.clone();
            let tasks_ready = tasks_ready.clone();
            let workspace = Arc::clone(&workspace);
            workers.push(tokio::spawn(async move {
                loop {
                    match tasks_ready.recv() {
                        Ok(task) => {
                            let result = run_task(task.lookup(&workspace), &workspace)
                                .await
                                .map(|_| task.clone());
                            task_finished.send(result.map_err(|_| task.clone())).ok();
                        }
                        Err(_) => return,
                    }
                }
            }));
        }

        // Start a number of workers that read from a queue.
        // Each worker needs to read from tasks_ready.
        // (Exit if it has been disconnected)
        // - Check if the inputs for the given task have changed.
        //   - If not, skip and record done.
        // If yes, do the task.
        // Send a message on task_finished so the worker knows what to do.
    });

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

    /*
       let files_changed = match opts.since {
           Some(since) => git::files_changed(git::Mode::Feature(since))?,
           None => {
               // In this case we probably need to know the hashes of all relevant
               // inputs.
               todo!()
           }
       };

       let repo_root = git::repo_root().expect("need to find repo root");
       let repo_root = repo_root.as_path();

       let projects_changed = files_changed
           .into_iter()
           .map(|p| repo_root.join(p))
           .flat_map(|file| {
               workspace
                   .projects()
                   .filter(|project| file.starts_with(&project.root))
                   .collect::<Vec<_>>()
           })
           .collect::<HashSet<_>>();

       let projects_affected = projects_changed
           .into_iter()
           .flat_map(|p| workspace.graph.walk_project_dependents(p.project_ref()))
           .collect::<HashSet<_>>();
    */

    Ok(())
}

fn filter_projects<'a>(
    workspace: &'a Workspace,
    filter: Option<ProjectFilter>,
) -> HashSet<&'a ProjectInfo> {
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

async fn run_task(task: &TaskInfo, workspace: &Workspace) -> Result<(), ()> {
    // TODO: want to determine whether this task needs to run based on its inputs/results
    // of its dependencies.

    for command in &task.commands {
        let mut args = command.split(' ');
        let command = args
            .next()
            .expect("there to be some content in a tasks command");

        let mut child = std::process::Command::new(command)
            .args(args)
            .current_dir(&workspace.lookup(&task.project).root)
            .spawn()
            .map_err(|_| ())?;

        // TODO: Do better stuff with output, currently just piped to stdout

        if !child.wait().map_err(|_| ())?.success() {
            return Err(());
        }
    }

    Ok(())
}
