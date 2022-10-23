use miette::{GraphicalReportHandler, GraphicalTheme};

use super::*;

#[test]
fn test_load_config_from_cwd() {
    insta::assert_debug_snapshot!(load_config_from_path("sample-monorepo/".into()));
}

#[test]
fn test_malformed_project_file() {
    let err =
        load_config_from_path("src/config/loader/test-data/malformed-project/".into()).unwrap_err();

    let mut report = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .with_width(80)
        .render_report(&mut report, err.as_ref())
        .unwrap();

    insta::assert_display_snapshot!(report);
}

#[test]
fn test_missing_task_file_error() {
    let err =
        load_config_from_path("src/config/loader/test-data/missing-task-file".into()).unwrap_err();

    let mut report = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .with_width(80)
        .render_report(&mut report, err.as_ref())
        .unwrap();

    insta::assert_display_snapshot!(report);
}
