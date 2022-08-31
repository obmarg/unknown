use std::{fs::File, io::Read};

use super::*;

#[test]
fn test_can_load_project_file() {
    let mut str_data = String::new();
    File::open("config-examples/project.kdl")
        .unwrap()
        .read_to_string(&mut str_data);

    insta::assert_debug_snapshot!(project_from_str(&str_data))
}
