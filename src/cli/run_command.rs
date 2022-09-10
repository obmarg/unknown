use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    process::Stdio,
    sync::Arc,
};

use futures::{stream::FuturesUnordered, StreamExt};
use tabled::{Table, Tabled};
use tokio::{io::AsyncReadExt, runtime::Runtime};

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

    let rt = Runtime::new().expect("to be able to start tokio runtime");
    rt.block_on(async {
        let tasks = tasks.clone();
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

            currently_running.push(tokio::spawn(async move {
                let result = run_task(task.lookup(&workspace), &workspace)
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

                            currently_running.push(tokio::spawn(async move {
                                let result = run_task(task.lookup(&workspace), &workspace)
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

        let mut child = tokio::process::Command::new(command)
            .args(args)
            .current_dir(&workspace.lookup(&task.project).root)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ())?;

        let mut child_stdout = child.stdout.take().expect("to get the stdout of a child");
        let mut child_stderr = child.stderr.take().expect("to get the stdout of a child");

        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];

        // TODO: Ideally want to colourise & align this, but I think that can wait.
        // Think this can also be abstracted.
        let annotation = format!("{}::{}: ", task.project.name(), task.name);
        let mut stdout = AnnotatedWrite::new(&annotation, std::io::stdout());
        let mut stderr = AnnotatedWrite::new(&annotation, std::io::stderr());

        // TODO: Move this block into some kind of extension trait/function
        loop {
            tokio::select! {
                stdout_read = child_stdout.read(&mut stdout_buf) => {
                    match stdout_read {
                        Ok(len) => {
                            stdout.write_all(&stdout_buf[0..len]).unwrap();
                        }
                        Err(_) => {
                            return Err(());
                        }
                    }
                },

                stderr_read = child_stderr.read(&mut stderr_buf) => {
                    match stderr_read {
                        Ok(len) => {
                            stderr.write_all(&stderr_buf[0..len]).unwrap();
                        }
                        Err(_) => {
                            return Err(());
                        }
                    }
                }

                result = child.wait() => {
                    // Done so... do something?
                    // For now just return tbh.
                    // At the very least I need to check return codes.
                    return Ok(());
                }
            }
        }

        // let stdout = std::io::BufReader::new(child.stdout.expect("to get the stdout of a child"));
        // let stderr = std::io::BufReader::new(child.stderr.expect("to get the stderr of a child"));
        // while let Ok(None) = child.try_wait() {
        //     use std::io::prelude;
        // }
        // TODO: Do better stuff with output, currently just piped to stdout
    }

    Ok(())
}

struct AnnotatedWrite<'a, W> {
    annotation: &'a str,
    inner: W,
    next_needs_annotated: bool,
    newline: u8,
}

impl<W> AnnotatedWrite<'_, W> {
    fn new<'a>(annotation: &'a str, inner: W) -> AnnotatedWrite<'a, W> {
        AnnotatedWrite {
            annotation,
            inner,
            next_needs_annotated: true,
            newline: u8::try_from('\n').unwrap(),
        }
    }
}

impl<W> std::io::Write for AnnotatedWrite<'_, W>
where
    W: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_used = 0;
        if self.next_needs_annotated {
            // TODO: Write annotation.
            self.next_needs_annotated = false;
            self.inner.write_all(self.annotation.as_bytes())?;
        }
        let mut chunks = buf.split_inclusive(|c| *c == self.newline).peekable();
        while let Some(chunk) = chunks.next() {
            bytes_used += self.inner.write(chunk)?;
            if chunks.peek().is_some() {
                self.inner.write_all(self.annotation.as_bytes())?;
            } else if chunk.ends_with(&[self.newline]) {
                self.next_needs_annotated = true;
            }
        }
        Ok(bytes_used)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
