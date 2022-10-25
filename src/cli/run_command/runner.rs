use std::{collections::HashMap, process::Stdio, sync::Arc};

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::task::{block_in_place, JoinHandle};

use crate::{
    git,
    hashing::{hash_task_inputs, Hash, HashRegistry},
    workspace::{TaskInfo, TaskRef, Workspace},
};

use super::{child_ext::ChildExt, output::CommandOutput, FinishedTask, TaskError, TaskOutcome};

pub(super) struct TaskRunner {
    currently_running: FuturesUnordered<JoinHandle<FinishedTask>>,
    workspace: Arc<Workspace>,
    since: Option<String>,
    outputs: HashMap<TaskRef, CommandOutput>,
    hash_registry: Arc<HashRegistry>,
    outcomes: HashMap<TaskRef, SimplifiedOutcome>,
}

enum SimplifiedOutcome {
    Skipped,
    Succesful,
    Failed,
}

impl TaskRunner {
    pub fn new(
        workspace: &Arc<Workspace>,
        since: Option<String>,
        outputs: HashMap<TaskRef, CommandOutput>,
        hash_registry: &Arc<HashRegistry>,
    ) -> TaskRunner {
        TaskRunner {
            currently_running: FuturesUnordered::new(),
            workspace: Arc::clone(workspace),
            outputs,
            since,
            hash_registry: Arc::clone(hash_registry),
            outcomes: HashMap::new(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn start_task(&mut self, task_ref: TaskRef) {
        let workspace = Arc::clone(&self.workspace);
        let since = self.since.clone();
        let hash_registry = Arc::clone(&self.hash_registry);
        let output = self
            .outputs
            .remove(&task_ref)
            .expect("a CommandOutput to exist for every task");

        let dependency_outcome = self.dependency_outcome(&task_ref);

        self.currently_running.push(tokio::spawn(async move {
            let res = run_task(
                task_ref.lookup(&workspace),
                &workspace,
                output,
                since,
                &hash_registry,
                dependency_outcome,
            )
            .await;

            FinishedTask {
                task_ref,
                outcome: match res {
                    Err(e) => TaskOutcome::Failed(e),
                    Ok(outcome) => outcome,
                },
            }
        }));
    }

    pub async fn next_finished(&mut self) -> Option<FinishedTask> {
        match self.currently_running.next().await {
            Some(Ok(finished)) => {
                self.outcomes.insert(
                    finished.task_ref.clone(),
                    SimplifiedOutcome::from_task_outcome(&finished.outcome),
                );
                Some(finished)
            }
            Some(Err(_)) => {
                // TODO: log this at least, maybe do something better?
                None
            }
            None => None,
        }
    }

    #[tracing::instrument(level = "debug" skip(self))]
    fn dependency_outcome(&self, task_ref: &TaskRef) -> OutcomeSummary {
        let succesful_tasks = task_ref
            .direct_dependencies(&self.workspace)
            .into_iter()
            .map(|dep| self.outcomes.get(&dep))
            .any(|t| matches!(t, Some(SimplifiedOutcome::Succesful)));

        if succesful_tasks {
            OutcomeSummary::SomeChange
        } else {
            OutcomeSummary::NoChange
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum OutcomeSummary {
    NoChange,
    SomeChange,
}

#[tracing::instrument(
    fields(task = %task.task_ref())
    skip(task, workspace, output, hash_registry)
)]
async fn run_task(
    task: &TaskInfo,
    workspace: &Workspace,
    output: CommandOutput,
    since: Option<String>,
    hash_registry: &HashRegistry,
    dependency_outcome: OutcomeSummary,
) -> Result<TaskOutcome, TaskError> {
    let (should_run, input_hash) =
        block_in_place(|| should_task_run(task, workspace, since, hash_registry))?;

    if !should_run && dependency_outcome == OutcomeSummary::NoChange {
        tracing::info!("Skipping task");
        return Ok(TaskOutcome::Skipped);
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
            .map_err(|_| TaskError::OutputError())?;
    }

    if let Some(input_hash) = input_hash {
        hash_registry.update_input_hash(task.task_ref(), input_hash);
    }

    Ok(TaskOutcome::Succesful)
}

#[tracing::instrument(
    fields(task = %task.task_ref())
    skip(task, workspace, hash_registry))
]
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
                .zip(new_hash)
                .map(|(last_hash, new_hash)| *last_hash != new_hash)
                .unwrap_or(true);

            Ok((should_run, new_hash))
        }
    }
}

impl SimplifiedOutcome {
    fn from_task_outcome(outcome: &TaskOutcome) -> SimplifiedOutcome {
        match outcome {
            TaskOutcome::Skipped => SimplifiedOutcome::Skipped,
            TaskOutcome::Succesful => SimplifiedOutcome::Succesful,
            TaskOutcome::Failed(_) => SimplifiedOutcome::Failed,
        }
    }
}
