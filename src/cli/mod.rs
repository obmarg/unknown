use clap::Parser;

use crate::{config::load_config_from_cwd, workspace::Workspace};

mod changed_command;
mod filters;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Reports which projects have changed in the repository, including dependants that may be affected
    Changed(changed_command::ChangedOpts),
}

pub fn run() -> miette::Result<()> {
    let opts = Cli::parse();
    let workspace = load_workspace()?;

    match opts.command {
        Command::Changed(command_opts) => changed_command::run(workspace, command_opts),
    }
}

fn load_workspace() -> Result<Workspace, miette::Report> {
    let (workspace_file, project_files) = load_config_from_cwd()?;

    // TODO: workspace::new should return an error probably
    Ok(Workspace::new(workspace_file, project_files))
}
