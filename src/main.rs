use std::path::{Path, PathBuf};

mod config;

fn main() {
    // So, suboptimal startup approach:
    // Walk the parent dir tree till we find a project.kdl
    // - If this is a project, keep walking till we find
    //   a workspace.
    // Read that workspace file.
    // Use the globs within to scan for any project files.
    // Read all the project files.
    //
    // Although ideally we only want to do this if a command
    // that requires this data has been called.
    // To be fair, that'll be most of them.
    //
    // May need a way to be smarter for speed purposes.
    // If projects were referred to by their paths that could
    // speed things up significantly.
    // But depends how slow it actually is to read all these project
    // files.  Probably an over optimisation initially.
    println!("Hello, world!");
}

fn find_workspace_file() -> Option<PathBuf> {
    let mut current_path = std::env::current_dir().ok()?;

    while current_path.parent().is_some() {
        current_path.push("workspace.kdl");
        if current_path.exists() {
            return Some(current_path);
        }
        current_path.pop();
        current_path.pop();
    }

    None
}
