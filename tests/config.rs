// TODO: At some point use libtest-mimic to make this smarter?  CBA now.
use assert_cmd::Command;

#[test]
fn task_require_non_ancestor() {
    test_failing_config("task_require_non_ancestor");
}

#[test]
fn task_require_unknown_project() {
    test_failing_config("task_require_unknown_project");
}

fn test_failing_config(name: &str) {
    let mut cmd = Command::cargo_bin("unknown").unwrap();
    cmd.arg("projects");
    cmd.current_dir(format!("tests/config-test-cases/{name}"));

    let assert = cmd.assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    insta::assert_snapshot!(format!("{name}.stdout"), stdout.as_ref());
    insta::assert_snapshot!(format!("{name}.stderr"), stderr.as_ref());
}
