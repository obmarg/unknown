use super::*;

mod hash_file_inputs {
    use similar_asserts::assert_eq;

    use crate::test_files::TestFiles;

    use super::*;

    #[test]
    fn test_file_hashes_are_consistent() {
        let files = TestFiles::new()
            .with_file("project.kdl", r#"project "hello""#)
            .with_file("blah.txt", r#"hello world"#);

        let mut first_hashes = Vec::new();
        let mut second_hashes = Vec::new();

        let globs = &[Glob::new("*").unwrap()];

        hash_file_inputs(&files.root().into(), globs, &mut first_hashes).unwrap();
        hash_file_inputs(&files.root().into(), globs, &mut second_hashes).unwrap();

        assert_eq!(first_hashes, second_hashes)
    }

    #[test]
    fn test_file_hashes_detect_changes() {
        let mut files = TestFiles::new()
            .with_file("project.kdl", r#"project "hello""#)
            .with_file("blah.txt", r#"hello world"#);

        let mut first_hashes = Vec::new();
        let mut second_hashes = Vec::new();

        let globs = &[Glob::new("*").unwrap()];

        hash_file_inputs(&files.root().into(), globs, &mut first_hashes).unwrap();

        files.add_file("test.txt", "");

        hash_file_inputs(&files.root().into(), globs, &mut second_hashes).unwrap();

        assert_ne!(first_hashes, second_hashes)
    }

    #[test]
    fn test_file_hashes_only_hashes_glob_matches() {
        let mut files = TestFiles::new().with_file("src/hello", r#"project "hello""#);

        let mut first_hashes = Vec::new();
        let mut second_hashes = Vec::new();

        let globs = &[Glob::new("src/*").unwrap()];

        hash_file_inputs(&files.root().into(), globs, &mut first_hashes).unwrap();

        // Add a file that does not match our glob
        files.add_file("test.txt", "");

        hash_file_inputs(&files.root().into(), globs, &mut second_hashes).unwrap();

        assert_eq!(first_hashes, second_hashes)
    }
}
