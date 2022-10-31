use miette::{GraphicalReportHandler, GraphicalTheme};

use crate::test_files::TestFiles;

use super::*;

#[test]
fn test_load_config_from_cwd() {
    insta::assert_debug_snapshot!(load_config_from_path("sample-monorepo/".into()));
}

#[test]
fn test_malformed_project_file() {
    let test_files = TestFiles::new()
        .with_file(
            "workspace.kdl",
            r#"
                name "my-workspace"
                project_path "**"
            "#,
        )
        .with_file(
            "project/project.kdl",
            r#"
                tasks {
                    import "missing.nabs"
                }
            "#,
        );

    let err = load_config_from_path(test_files.root().into()).unwrap_err();

    insta::assert_display_snapshot!(render_miette(err, &test_files));
}

#[test]
fn test_missing_task_file_error() {
    let test_files = TestFiles::new()
        .with_file(
            "workspace.kdl",
            r#"
                name "my-workspace"
                project_path "**"
            "#,
        )
        .with_file(
            "project/project.kdl",
            r#"
            project "missing-task-file-project"

            tasks {
                import "missing.nabs"
            }
        "#,
        );

    let err = load_config_from_path(test_files.root().into()).unwrap_err();

    insta::assert_display_snapshot!(render_miette(err, &test_files));
}

#[test]
fn test_dependency_from_out_of_workspace() {
    let test_files = TestFiles::new()
        .with_file("project.kdl", r#"project "proj-outside-workspace"#)
        .with_file(
            "workspace/workspace.kdl",
            r#"
                name "test"

                project_path "*"
            "#,
        )
        .with_file(
            "workspace/project/project.kdl",
            r#"
                project "my_project"
                dependencies {
                    project "../../"
                }
            "#,
        );

    let err =
        load_config_from_path(Utf8PathBuf::from(test_files.root()).join("workspace")).unwrap_err();

    insta::assert_display_snapshot!(render_miette(err, &test_files));
}

#[test]
fn test_wont_import_tasks_from_out_of_workspace() {
    let test_files = TestFiles::new()
        .with_file("task.kdl", "")
        .with_file(
            "workspace/workspace.kdl",
            r#"
                name "test"

                project_path "*"
            "#,
        )
        .with_file(
            "workspace/project/project.kdl",
            r#"
                project "my_project"
                tasks {
                    import "../../task.kdl"
                }
            "#,
        );
    let err =
        load_config_from_path(Utf8PathBuf::from(test_files.root()).join("workspace")).unwrap_err();

    insta::assert_display_snapshot!(render_miette(err, &test_files));
}

fn render_miette(e: miette::Report, test_files: &TestFiles) -> String {
    let mut report = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut report, e.as_ref())
        .unwrap();

    // We don't want any randomly generated absolute paths in our error, so
    // find the root and replace it in the error string with /
    let file_root = Utf8PathBuf::from(test_files.root())
        .canonicalize_utf8()
        .unwrap();

    report.replace(file_root.as_str(), "/")
}
