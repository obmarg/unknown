use crate::workspace::Workspace;

#[derive(clap::Parser)]
pub struct GraphOpts {}

pub fn run(workspace: Workspace, _opts: GraphOpts) -> miette::Result<()> {
    println!("{:?}", workspace.graph.dot());

    Ok(())
}
