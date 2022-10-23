use clap::Parser;

use crate::{config::load_config_from_path, workspace::Workspace};

mod changed_command;
mod filters;
mod run_command;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Reports which projects have changed in the repository, including dependants that may be affected
    Changed(changed_command::ChangedOpts),
    Run(run_command::RunOpts),
}

pub fn run() -> miette::Result<()> {
    let opts = Cli::parse();
    let workspace = load_workspace()?;

    match opts.command {
        Command::Changed(command_opts) => changed_command::run(workspace, command_opts),
        Command::Run(command_opts) => run_command::run(workspace, command_opts),
    }
}

fn load_workspace() -> Result<Workspace, miette::Report> {
    let (workspace_file, project_files) = load_config_from_path(
        std::env::current_dir().expect("couldn't determine current directory"),
    )?;

    // TODO: workspace::new should return an error probably
    Ok(Workspace::new(workspace_file, project_files))
}
