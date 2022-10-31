use std::{fs::File, io::Read};

use camino::Utf8PathBuf;

use super::parsing::{parse_project_file, parse_task_file, parse_workspace_file};

#[test]
fn test_can_load_project_file() {
    let mut str_data = String::new();
    File::open("config-examples/project.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    insta::assert_debug_snapshot!(parse_project_file(
        &Utf8PathBuf::from("blah/project.kdl"),
        &dbg!(str_data)
    )
    .map_err(|e| miette::Report::new(e.0)))
}

#[test]
fn test_can_load_workspace_file() {
    let mut str_data = String::new();
    File::open("config-examples/workspace.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    insta::assert_debug_snapshot!(parse_workspace_file(
        &Utf8PathBuf::from("blah/workspace.kdl"),
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

    insta::assert_debug_snapshot!(
        parse_task_file(&Utf8PathBuf::from("blah/task.kdl"), &str_data)
            .map_err(|e| miette::Report::new(e.0))
    )
}
