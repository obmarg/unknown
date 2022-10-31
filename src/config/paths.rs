use camino::{Utf8Path, Utf8PathBuf};
use knuffel::{
    ast::Literal, decode::Kind, errors::DecodeError, span::Spanned, traits::ErrorSpan, DecodeScalar,
};

#[derive(Clone, Debug)]
pub struct ConfigPath {
    span: miette::SourceSpan,
    inner: PathInner,
}

#[derive(Clone, Debug)]
pub enum PathInner {
    Raw(Utf8PathBuf),
    Normalised(NormalisedPath),
}

impl Default for PathInner {
    fn default() -> Self {
        PathInner::Raw(Utf8PathBuf::new())
    }
}

impl ConfigPath {
    // pub fn span(&self) -> miette::SourceSpan {
    //     self.span.clone()
    // }

    pub fn into_normalised(self) -> Option<NormalisedPath> {
        match self.inner {
            PathInner::Raw(_) => None,
            PathInner::Normalised(inner) => Some(inner),
        }
    }

    pub fn normalise_relative_to(
        &mut self,
        relative_to: &NormalisedPath,
    ) -> Result<(), ConfigPathValidationError> {
        let PathInner::Raw(path) = std::mem::take(&mut self.inner) else {
            panic!("Tried to normalise a ConfigPath twice");
        };
        self.inner = PathInner::Normalised(
            relative_to
                .normalise_relative(path)
                .map_err(|e| ConfigPathValidationError::new(e, self.span))?,
        );
        Ok(())
    }
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum ConfigPathValidationError {
    #[error("Path doesn't seem to exist: {0}")]
    #[diagnostic(help(
        "paths can be relative to the current file or absolute to the root of the workspace"
    ))]
    FileNotFound(
        Utf8PathBuf,
        #[label("the path is referenced here")] miette::SourceSpan,
    ),
    #[error("Permission denied on path: {0}")]
    #[help("Make sure you have read permisions to this path")]
    PermissionDenied(
        Utf8PathBuf,
        #[label("the path is referenced here")] miette::SourceSpan,
    ),
    #[error("An unexpected error occurred when trying to read a file: {0}")]
    OtherIo(
        std::io::Error,
        #[label("the path is referenced here")] miette::SourceSpan,
    ),
    #[error("The provided path was not in the workspace: {0}")]
    #[help("all paths need to be descended from the directory containing your workspace.kdl")]
    PathNotInWorkspace(
        Utf8PathBuf,
        #[label("the path is referenced here")] miette::SourceSpan,
    ),
}

impl ConfigPathValidationError {
    pub fn new(err: PathError, span: miette::SourceSpan) -> ConfigPathValidationError {
        match err {
            PathError::FileNotFound(path) => ConfigPathValidationError::FileNotFound(path, span),
            PathError::PermissionDenied(path) => {
                ConfigPathValidationError::PermissionDenied(path, span)
            }
            PathError::OtherIo(err) => ConfigPathValidationError::OtherIo(err, span),
            PathError::PathNotInWorkspace(path) => {
                ConfigPathValidationError::PathNotInWorkspace(path, span)
            }
        }
    }
}

// impl std::fmt::Display for Path {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

impl<S> DecodeScalar<S> for ConfigPath
where
    S: ErrorSpan,
{
    fn type_check(
        _type_name: &Option<knuffel::span::Spanned<knuffel::ast::TypeName, S>>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) {
        // Not bothering with types for now...
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let Literal::String(s) = &**value else {
            let found =  match **value {
                Literal::Null => Kind::Null,
                Literal::Bool(_) => Kind::Bool,
                Literal::Int(_) => Kind::Int,
                Literal::Decimal(_) => Kind::Decimal,
                Literal::String(_) => panic!("this should be impossible")
            };
            return Err(DecodeError::ScalarKind {
                span: value.span().to_owned(),
                expected: Kind::String.into(),
                found
            });
        };

        let path = Utf8PathBuf::try_from(s.as_ref()).map_err(|error| DecodeError::Conversion {
            span: value.span().to_owned(),
            source: Box::new(error),
        })?;

        Ok(ConfigPath {
            span: value.span().to_owned().into(),
            inner: PathInner::Raw(path),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceRoot(Utf8PathBuf);

impl AsRef<Utf8Path> for WorkspaceRoot {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl From<WorkspaceRoot> for Utf8PathBuf {
    fn from(val: WorkspaceRoot) -> Self {
        val.0
    }
}

impl AsRef<std::path::Path> for WorkspaceRoot {
    fn as_ref(&self) -> &std::path::Path {
        self.0.as_ref()
    }
}

impl From<WorkspaceRoot> for NormalisedPath {
    fn from(val: WorkspaceRoot) -> Self {
        NormalisedPath {
            workspace_root: val,
            subpath: Utf8PathBuf::new(),
        }
    }
}

impl WorkspaceRoot {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        WorkspaceRoot(path.into())
    }

    pub fn normalise_subpath(
        &self,
        path: impl Into<Utf8PathBuf>,
    ) -> Result<NormalisedPath, PathError> {
        let path = path.into();
        let absolute = match path.is_absolute() {
            true => path,
            false => self.0.join(path),
        };

        let absolute = absolute
            .canonicalize_utf8()
            .map_err(|e| PathError::from_io_error(e, absolute))?;

        let subpath = absolute
            .strip_prefix(&self.0)
            .map_err(|_| PathError::PathNotInWorkspace(absolute.clone()))?
            .to_owned();

        Ok(NormalisedPath {
            workspace_root: self.clone(),
            subpath,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NormalisedPath {
    workspace_root: WorkspaceRoot,
    subpath: Utf8PathBuf,
}

impl std::fmt::Display for NormalisedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.subpath)
    }
}

impl NormalisedPath {
    pub fn full_path(&self) -> Utf8PathBuf {
        self.workspace_root.0.join(&self.subpath)
    }

    pub fn as_subpath(&self) -> &Utf8PathBuf {
        &self.subpath
    }

    pub fn parent(&self) -> Option<NormalisedPath> {
        self.subpath.parent().map(|subpath| NormalisedPath {
            workspace_root: self.workspace_root.clone(),
            subpath: subpath.to_owned(),
        })
    }

    // Normalises the provided path relative to self (or the root of the repo if path is absolute)
    fn normalise_relative(
        &self,
        path: impl Into<Utf8PathBuf>,
    ) -> Result<NormalisedPath, PathError> {
        let mut path = path.into();

        let base;
        match path.is_absolute() {
            true => {
                base = self.workspace_root.0.clone();
                path = Utf8PathBuf::from(path.as_str().strip_prefix('/').unwrap());
            }
            false => {
                base = self.workspace_root.0.join(&self.subpath);
            }
        };

        let absolute = base.join(path);
        let absolute = absolute
            .canonicalize_utf8()
            .map_err(|e| PathError::from_io_error(e, absolute))?;

        let subpath = absolute
            .strip_prefix(&self.workspace_root.0)
            .map_err(|_| PathError::PathNotInWorkspace(absolute.clone()))?
            .to_owned();

        Ok(NormalisedPath {
            workspace_root: self.workspace_root.clone(),
            subpath,
        })
    }
}

impl serde::Serialize for NormalisedPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.subpath.as_str())
    }
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum PathError {
    #[error("Could not find file: {0}")]
    FileNotFound(Utf8PathBuf),
    #[error("Permission denied on file: {0}")]
    PermissionDenied(Utf8PathBuf),
    #[error("An unexpected error occurred when trying to read a file: {0}")]
    OtherIo(std::io::Error),
    #[error("The provided path was not in the workspace: {0}")]
    PathNotInWorkspace(Utf8PathBuf),
}

impl PathError {
    fn from_io_error(e: std::io::Error, path: Utf8PathBuf) -> PathError {
        match e.kind() {
            std::io::ErrorKind::NotFound => PathError::FileNotFound(path),
            std::io::ErrorKind::PermissionDenied => PathError::PermissionDenied(path),
            _ => PathError::OtherIo(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(knuffel::Decode, Debug)]
    pub struct TestStruct {
        #[knuffel(children(name = "path"), unwrap(argument))]
        pub paths: Vec<ConfigPath>,
    }

    #[test]
    fn test_decoding_paths() {
        let result = knuffel::parse::<TestStruct>(
            "whatevs.txt",
            r#"
        path "hello"
        path "../hello/"
        path "/hello"
        "#,
        )
        .unwrap();

        insta::assert_debug_snapshot!(result, @r###"
        TestStruct {
            paths: [
                ConfigPath {
                    span: SourceSpan {
                        offset: SourceOffset(
                            14,
                        ),
                        length: SourceOffset(
                            7,
                        ),
                    },
                    inner: Raw(
                        "hello",
                    ),
                },
                ConfigPath {
                    span: SourceSpan {
                        offset: SourceOffset(
                            35,
                        ),
                        length: SourceOffset(
                            11,
                        ),
                    },
                    inner: Raw(
                        "../hello/",
                    ),
                },
                ConfigPath {
                    span: SourceSpan {
                        offset: SourceOffset(
                            60,
                        ),
                        length: SourceOffset(
                            8,
                        ),
                    },
                    inner: Raw(
                        "/hello",
                    ),
                },
            ],
        }
        "###);
    }
}
