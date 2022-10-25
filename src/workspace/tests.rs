use similar_asserts::assert_eq;

use crate::config::load_config_from_path;

use super::*;

#[test]
fn snapshot_sample_monorepo() {
    let (workspace, projects) = load_config_from_path("src/workspace/test-data/".into()).unwrap();

    insta::assert_debug_snapshot!(Workspace::new(workspace, projects))
}

#[test]
fn test_task_ref_direct_dependencies() {
    let workspace = a_workspace();

    let build_lib_ref = workspace
        .lookup_project("a-lib")
        .unwrap()
        .lookup_task("build")
        .unwrap()
        .task_ref();

    let build_project_ref = workspace
        .lookup_project("a-service")
        .unwrap()
        .lookup_task("build")
        .unwrap()
        .task_ref();

    assert_eq!(
        build_project_ref.direct_dependencies(&workspace),
        maplit::hashset! { build_lib_ref }
    )
}

fn a_workspace() -> Workspace {
    let (workspace, projects) = load_config_from_path("src/workspace/test-data/".into()).unwrap();
    Workspace::new(workspace, projects)
}
