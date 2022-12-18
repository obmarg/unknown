use std::{fs::File, io::Read};

use camino::Utf8PathBuf;
use miette::{GraphicalReportHandler, GraphicalTheme};

use super::{
    parsing::{parse_project_file, parse_task_file, parse_workspace_file},
    ConfigSource,
};

#[test]
fn test_can_load_project_file() {
    let mut str_data = String::new();
    File::open("config-examples/project.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();
    let source = ConfigSource::new("blah/project.kdl", str_data);

    insta::assert_debug_snapshot!(parse_project_file(&source).map_err(|e| miette::Report::new(e.0)))
}

#[test]
fn test_can_load_workspace_file() {
    let mut str_data = String::new();
    File::open("config-examples/workspace.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    match parse_workspace_file(
        &Utf8PathBuf::from("config-examples/workspace.kdl"),
        &str_data,
    ) {
        Ok(parsed) => {
            insta::assert_debug_snapshot!(parsed)
        }
        Err(e) => {
            panic!("{}", render_miette(miette::Report::new(e.0)))
        }
    }
}

#[test]
fn test_can_load_task_file() {
    let mut str_data = String::new();
    File::open("config-examples/task.kdl")
        .unwrap()
        .read_to_string(&mut str_data)
        .unwrap();

    match parse_task_file(&Utf8PathBuf::from("config-examples/task.kdl"), &str_data) {
        Ok(parsed) => {
            insta::assert_debug_snapshot!(parsed)
        }
        Err(e) => {
            panic!("{}", render_miette(miette::Report::new(e.0)))
        }
    }
}

fn render_miette(e: miette::Report) -> String {
    let mut report = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut report, e.as_ref())
        .unwrap();

    report
}
