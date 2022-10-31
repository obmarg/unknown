use camino::{Utf8Path, Utf8PathBuf};
use tempfile::TempDir;

use crate::config::WorkspaceRoot;

pub struct TestFiles {
    dir: TempDir,
}

impl TestFiles {
    pub fn new() -> Self {
        TestFiles {
            dir: TempDir::new().expect("to be able to create a temp dir"),
        }
    }

    pub fn add_file<Name: ?Sized + AsRef<str>, Contents: ?Sized + AsRef<str>>(
        &mut self,
        name: &Name,
        contents: &Contents,
    ) {
        let path = Utf8PathBuf::from(name);
        assert!(path.is_relative());

        let path = self.dir.path().join(path);
        std::fs::create_dir_all(path.parent().expect("path to have a parent"))
            .expect("to be able to create any dirs");

        let contents = unindent::unindent(contents.as_ref());

        std::fs::write(path, contents).expect("to be able to write a file")
    }

    pub fn with_file<Name: ?Sized + AsRef<str>, Contents: ?Sized + AsRef<str>>(
        mut self,
        name: &Name,
        contents: &Contents,
    ) -> Self {
        self.add_file(name, contents);
        self
    }

    pub fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::new(Utf8Path::from_path(self.dir.path()).unwrap().to_owned())
    }
}
