use assert_cmd::Command;
use assert_fs::{prelude::*, TempDir};
use similar_asserts::assert_eq;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_changed_command() -> Result<()> {
    let mut temp_dir = TempDir::new()?;

    // Note: Uncomment these if you're debugging
    // temp_dir = temp_dir.into_persistent();
    // println!("Not So Temp Dir: {temp_dir:?}");

    temp_dir
        .child("workspace.kdl")
        .write_str("name \"test_changed_command\"\n")?;
    temp_dir.child("service/project.kdl").write_str(
        r#"
      project "service"
      dependencies {
        project "/library"
      }
    "#,
    )?;
    temp_dir.child("service/file.txt").touch()?;
    temp_dir
        .child("service/nested/project.kdl")
        .write_str(r#"project "nested-service""#)?;
    temp_dir
        .child("library/project.kdl")
        .write_str(r#"project "library""#)?;

    let library_file = temp_dir.child("library/file.txt");
    let service_file = temp_dir.child("service/file.txt");
    let nested_service_file = temp_dir.child("service/nested/file.txt");

    library_file.touch()?;
    service_file.touch()?;
    nested_service_file.touch()?;

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

    assert_eq!(run_changed(&mut temp_dir)?, Vec::<String>::new());

    nested_service_file.write_str("update")?;
    assert_eq!(run_changed(&mut temp_dir)?, vec!["service/nested"]);

    service_file.write_str("update")?;
    assert_eq!(
        run_changed(&mut temp_dir)?,
        vec!["service", "service/nested"]
    );

    service_file.write_str("")?;
    nested_service_file.write_str("")?;
    assert_eq!(run_changed(&mut temp_dir)?, Vec::<String>::new());

    library_file.write_str("update")?;
    assert_eq!(run_changed(&mut temp_dir)?, vec!["library", "service"]);

    Ok(())
}

fn run_changed(dir: &mut TempDir) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Output {
        path: String,
    }

    let output = serde_json::from_slice::<Vec<Output>>(
        &Command::cargo_bin("unknown")?
            .args(["changed", "--format", "json"])
            .current_dir(dir)
            .ok()?
            .stdout,
    )?;

    Ok(output.into_iter().map(|o| o.path).collect())
}
