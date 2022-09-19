use camino::{Utf8Path, Utf8PathBuf};

#[derive(Eq, Debug, Clone)]
pub struct WorkspacePath {
    absolute: Utf8PathBuf,
    relative: Utf8PathBuf,
}

impl WorkspacePath {
    pub fn for_workspace(path: impl AsRef<std::path::Path>) -> Self {
        // TODO: Check if path is absolute
        WorkspacePath {
            absolute: Utf8Path::from_path(path.as_ref())
                .expect("a utf8 path")
                .to_owned(),
            relative: Utf8PathBuf::new(),
        }
    }

    pub fn subpath(&self, path: impl AsRef<std::path::Path>) -> Self {
        // TODO: Check if path is absolute
        let absolute = Utf8Path::from_path(path.as_ref())
            .expect("a utf8 path")
            .to_owned();

        let relative = absolute
            .strip_prefix(&self.absolute)
            .expect("path provided to be a subpath")
            .to_owned();

        WorkspacePath { absolute, relative }
    }
}

impl std::hash::Hash for WorkspacePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We ignore relative for the sake of hashing
        self.absolute.hash(state);
    }
}

impl std::cmp::PartialEq for WorkspacePath {
    fn eq(&self, other: &Self) -> bool {
        // We ignore relative for the sake of comparisons
        self.absolute == other.absolute
    }
}

impl AsRef<std::path::Path> for WorkspacePath {
    fn as_ref(&self) -> &std::path::Path {
        self.absolute.as_ref()
    }
}

impl AsRef<Utf8Path> for WorkspacePath {
    fn as_ref(&self) -> &Utf8Path {
        self.absolute.as_ref()
    }
}

impl From<WorkspacePath> for Utf8PathBuf {
    fn from(val: WorkspacePath) -> Self {
        val.absolute
    }
}

impl std::fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.relative)
    }
}

impl serde::Serialize for WorkspacePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.relative.as_str())
    }
}
