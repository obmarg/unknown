use std::{fs::File, io::Read};

use super::*;

#[test]
fn test_can_load_project_file() {
    let mut str_data = String::new();
    File::open("config-examples/project.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    insta::assert_debug_snapshot!(
        parse_project_file(&PathBuf::from("blah/project.kdl"), &str_data)
            .map_err(|e| miette::Report::new(e.0))
    )
}

#[test]
fn test_can_load_workspace_file() {
    let mut str_data = String::new();
    File::open("config-examples/workspace.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    insta::assert_debug_snapshot!(parse_workspace_file(
        &PathBuf::from("blah/workspace.kdl"),
        &str_data
    )
    .map_err(|e| miette::Report::new(e.0)))
}

#[test]
fn test_can_load_task_file() {
    let mut str_data = String::new();
    File::open("config-examples/task.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    insta::assert_debug_snapshot!(parse_task_file(&PathBuf::from("blah/task.kdl"), &str_data)
        .map_err(|e| miette::Report::new(e.0)))
}
