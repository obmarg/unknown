use clap::Parser;

use crate::{config::load_config_from_cwd, workspace::Workspace};

mod changed_command;
mod opts;

pub fn run() -> miette::Result<()> {
    let opts = opts::Cli::parse();
    let workspace = load_workspace()?;

    match opts.command {
        opts::Command::Changed(command_opts) => changed_command::run(workspace, command_opts),
    }
}

fn load_workspace() -> Result<Workspace, miette::Report> {
    let (workspace_file, project_files) = load_config_from_cwd()?;

    // TODO: workspace::new should return an error probably
    Ok(Workspace::new(workspace_file, project_files))
}
