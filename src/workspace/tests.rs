use insta::{dynamic_redaction, Settings};
use similar_asserts::assert_eq;

use crate::config::load_config_from_path;

use super::*;

#[test]
fn snapshot_sample_monorepo() {
    let config = load_config_from_path("src/workspace/test-data/".into()).unwrap();

    let mut workspace = Workspace::new(config.workspace_file);
    workspace.add_projects(config.project_files).unwrap();

    let mut settings = Settings::clone_current();
    settings.add_redaction(
        ".**.root_path",
        dynamic_redaction(|value, _| {
            // assert_eq!(path.to_string(), ".info.root_path");
            assert_eq!(
                value.as_str().unwrap().contains("src/workspace/test-data/"),
                true
            );
            "src/workspace/test-data/"
        }),
    );
    settings.set_sort_maps(true);
    settings.sort_selector(".**.project_indices");
    settings.sort_selector(".**.task_indices");
    // settings.sort_selector(".**.nodes");
    settings.sort_selector(".**.edges");
    settings.sort_selector(".**.task_map");
    settings.sort_selector(".**.project_map");

    settings.bind(
        || insta::assert_json_snapshot!(workspace, {".**.nodes" => insta::sorted_redaction(), ".**.edges" => insta::sorted_redaction()}),
    )
}

#[test]
fn test_task_ref_direct_dependencies() {
    let workspace = a_workspace();

    let build_lib_ref = workspace
        .project_at_path("projects/a-lib")
        .unwrap()
        .lookup_task("build", &workspace)
        .unwrap()
        .task_ref();

    let build_project_ref = workspace
        .project_at_path("projects/a-service")
        .unwrap()
        .lookup_task("build", &workspace)
        .unwrap()
        .task_ref();

    assert_eq!(
        build_project_ref.direct_dependencies(&workspace),
        maplit::hashset! { build_lib_ref }
    )
}

fn a_workspace() -> Workspace {
    let config = load_config_from_path("src/workspace/test-data/".into()).unwrap();

    let mut workspace = Workspace::new(config.workspace_file);
    workspace.add_projects(config.project_files).unwrap();
    workspace
}
