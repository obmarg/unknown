use camino::Utf8PathBuf;
use clap::Parser;

use crate::{
    config::{load_config_from_path, Validator},
    workspace::Workspace,
};

mod changed_command;
mod filters;
mod git_commands;
mod graph_command;
mod projects_command;
mod run_command;
mod tasks_command;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Reports which projects have changed in the repository including dependants
    /// that may be affected
    Changed(changed_command::ChangedOpts),
    /// Runs a command (WIP)
    Run(run_command::RunOpts),
    /// Prints information about projects in the workspace (WIP)
    Projects(projects_command::ProjectsOpts),
    /// Prints information about tasks in the workspace (WIP)
    Tasks(tasks_command::TasksOpts),
    /// Prints a graph of the workspace in dot format
    Graph(graph_command::GraphOpts),
    /// Subcommands for manipulating a sparse-checkout git repository
    #[clap(subcommand)]
    Git(git_commands::GitCommand),
}

pub fn run() -> miette::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Cli::parse();
    let workspace = load_workspace()?;

    match opts.command {
        Command::Changed(command_opts) => changed_command::run(workspace, command_opts),
        Command::Run(command_opts) => run_command::run(workspace, command_opts),
        Command::Projects(command_opts) => projects_command::run(workspace, command_opts),
        Command::Tasks(command_opts) => tasks_command::run(workspace, command_opts),
        Command::Graph(command_opts) => graph_command::run(workspace, command_opts),
        Command::Git(command_opts) => git_commands::run(workspace, command_opts),
    }
}

fn load_workspace() -> Result<Workspace, miette::Report> {
    let config = load_config_from_path(
        Utf8PathBuf::try_from(
            std::env::current_dir().expect("couldn't determine current directory"),
        )
        .expect("the current directory to be a utf8 path"),
    )?;

    let mut workspace = Workspace::new(config.workspace_file);

    workspace.add_projects(config.project_files)?;

    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
