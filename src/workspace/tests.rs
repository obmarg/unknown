use similar_asserts::assert_eq;

use crate::config::load_config_from_path;

use super::*;

#[test]
fn snapshot_sample_monorepo() {
    let config = load_config_from_path("src/workspace/test-data/".into()).unwrap();

    let mut workspace = Workspace::new(config.workspace_file);
    workspace.add_projects(config.project_files).unwrap();

    insta::assert_debug_snapshot!(workspace)
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
