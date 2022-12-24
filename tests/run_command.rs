use std::ffi;

use assert_cmd::Command;
use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
use similar_asserts::assert_eq;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn no_changes_git() -> Result<()> {
    let workspace = setup_workspace()?;

    insta::assert_snapshot!(
        "no_changes_git.stdout",
        workspace.run(["run", "build", "--since", "HEAD"])
    );

    Ok(())
}

#[test]
fn building_service() {
    let workspace = setup_workspace().unwrap();

    // Now make sure we're doing nothing when we re-run
    insta::assert_snapshot!(
        "building_service.stdout",
        workspace.run(["run", "build", "--filter", "service"])
    );
}

#[test]
fn doesnt_rerun_nested_if_inputs_unchanged() {
    let workspace = setup_workspace().unwrap();

    // Setup our hashes
    workspace.run(["run", "build"]);

    // Now make sure nested doesn't get rebuilt
    insta::assert_snapshot!(
        "doesnt_rerun_nested_if_inputs_unchanged.stdout",
        workspace.run(["run", "build"])
    );
}

#[test]
fn pulls_in_dependencies() {
    let workspace = setup_workspace().unwrap();
    insta::assert_snapshot!(
        "pulls_in_dependencies.stdout",
        workspace.run(["run", "build", "--filter", "service"])
    );
}

#[test]
fn failures_stop_execution() {
    let workspace = setup_workspace().unwrap();
    let (stdout, stderr) = workspace.run_failure(["run", "fail", "--filter", "service"]);
    insta::assert_snapshot!("failures_stop_execution.stdout", stdout);
    insta::assert_snapshot!("failures_stop_execution.stderr", stderr);
}

#[test]
fn nested_file_change_with_since() {
    let workspace = setup_workspace().unwrap();

    workspace.nested_service_file.write_str("hello").unwrap();

    // This should build _just_ the nested service, despite the fact that the
    // changed file is also nested under service
    insta::assert_snapshot!(
        "nested_file_change_with_since.stdout",
        workspace.run(["run", "build", "--since", "HEAD"])
    );
}

#[test]
fn change_to_a_non_input_file_is_ignored_with_since() {
    let workspace = setup_workspace().unwrap();

    workspace
        .non_input_nested_service_file
        .write_str("hello")
        .unwrap();

    assert_eq!(workspace.run(["run", "build", "--since", "HEAD"]), "");
}

#[test]
fn nested_file_change_with_hashes() {
    let workspace = setup_workspace().unwrap();

    // setup the hashes.
    workspace.run(["run", "build"]);

    workspace.nested_service_file.write_str("hello").unwrap();

    // TOOD: don't approve the snapshot on this till it works.

    // This should build _just_ the nested service, despite the fact that the
    // changed file is also nested under service
    insta::assert_snapshot!(
        "nested_file_change_with_hashes.stdout",
        workspace.run(["run", "build", "--filter", "nested,service"])
    );
}

#[test]
fn change_to_a_non_input_file_is_ignored_with_hashes() {
    let workspace = setup_workspace().unwrap();

    // setup the hashes.
    workspace.run(["run", "build"]);

    workspace
        .non_input_nested_service_file
        .write_str("hello")
        .unwrap();

    assert_eq!(workspace.run(["run", "build", "--filter", "nested"]), "");
}

#[test]
fn filter_implicitly_set_to_current_folders_project() {
    let workspace = setup_workspace().unwrap();

    let output = Command::cargo_bin("unknown")
        .unwrap()
        .args(["run", "build"])
        .current_dir(&workspace.root.child("library"))
        .ok()
        .unwrap();

    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(
        "filter_implicitly_set_to_current_folders_project.stdout",
        stdout
    );
}

#[allow(dead_code)]
struct TestWorkspace {
    root: TempDir,
    library_file: ChildPath,
    service_file: ChildPath,
    nested_service_file: ChildPath,
    non_input_nested_service_file: ChildPath,
}

impl TestWorkspace {
    fn run<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<ffi::OsStr>,
    {
        let output = Command::cargo_bin("unknown")
            .unwrap()
            .args(args)
            .current_dir(&self.root)
            .ok()
            .unwrap();

        assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
        String::from_utf8(output.stdout).unwrap()
    }

    fn run_failure<I, S>(&self, args: I) -> (String, String)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<ffi::OsStr>,
    {
        let output = Command::cargo_bin("unknown")
            .unwrap()
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap();

        (
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

fn setup_workspace() -> Result<TestWorkspace> {
    let mut temp_dir = TempDir::new()?;

    // Note: Uncomment these if you're debugging
    temp_dir = temp_dir.into_persistent();
    println!("Not So Temp Dir: {temp_dir:?}");

    temp_dir
        .child("workspace.kdl")
        .write_str("name \"test_changed_command\"\n")?;
    temp_dir.child("service/project.kdl").write_str(
        r##"
            project "service"
            dependencies {
                project "/library"
            }

            tasks {
                task "build" {
                    requires "build" in="^self"
                    command r#"echo "Building Service""#
                    inputs {
                        path "**"
                    }
                }
                task "fail" {
                    requires "fail" in="library"
                    command r#"echo "If you see this, the test has failed""#
                }
            }
        "##,
    )?;
    temp_dir.child("service/nested/project.kdl").write_str(
        r##"
            project "nested"

            tasks {
                task "build" {
                    command r#"echo "Building Nested""#
                    inputs {
                        // TODO: Ok, something up with this file specification
                        path "file.txt"
                    }
                }
            }
        "##,
    )?;
    temp_dir.child("library/project.kdl").write_str(
        r##"
            project "library"

            tasks {
                task "build" {
                    command r#"echo "Building Library""#
                    inputs {
                        path "**"
                    }
                }
                task "fail" {
                    // Just some nonsense command that'll output an error code.
                    command "git fail"
                }
            }
        "##,
    )?;

    let library_file = temp_dir.child("library/file.txt");
    let service_file = temp_dir.child("service/file.txt");
    let nested_service_file = temp_dir.child("service/nested/file.txt");
    let non_input_nested_service_file = temp_dir.child("service/nested/other.txt");

    library_file.touch()?;
    service_file.touch()?;
    nested_service_file.touch()?;
    non_input_nested_service_file.touch()?;

    Command::new("git")
        .arg("init")
        .current_dir(&temp_dir)
        .ok()?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .ok()?;
    Command::new("git")
        .args(["commit", "-m", "whatevs"])
        .current_dir(&temp_dir)
        .ok()?;

    Ok(TestWorkspace {
        root: temp_dir,
        library_file,
        service_file,
        nested_service_file,
        non_input_nested_service_file,
    })
}
