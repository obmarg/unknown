use validated::Selection;

use crate::config::{
    paths::{ConfigPath, ConfigPathValidationError},
    spanned::{SourceSpanExt, Spanned, WithSpan},
    validated, Glob, WorkspaceRoot,
};

#[derive(knuffel::Decode, Debug, Default)]
pub struct TaskBlock {
    #[knuffel(children(name = "import"), unwrap(argument))]
    pub(in crate::config) imports: Vec<ConfigPath>,

    // #[knuffel(children(name = "import_template"), unwrap(argument))]
    // template_imports: Vec<String>,
    #[knuffel(children(name = "task"))]
    pub(in crate::config) tasks: Vec<TaskDefinition>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("A task file failed validation")]
pub enum TaskValidationError {
    InvalidPaths(#[related] Vec<ConfigPathValidationError>),
    MalformedRequire {
        span: miette::SourceSpan,
        message: String,
    },
}

#[derive(knuffel::Decode, Debug)]
pub struct TaskDefinition {
    #[knuffel(argument)]
    pub(super) name: String,

    #[knuffel(children(name = "command"), unwrap(argument))]
    pub(super) commands: Vec<String>,

    #[knuffel(children(name = "requires"))]
    pub(super) requires: Vec<TaskRequires>,

    #[knuffel(children(name = "inputs"))]
    pub(super) input_blocks: Vec<InputBlock>,
}

impl From<InputBlock> for validated::InputBlock {
    fn from(value: InputBlock) -> Self {
        validated::InputBlock {
            paths: value.paths,
            env_vars: value.env_vars,
            commands: value.commands,
        }
    }
}

#[derive(knuffel::Decode, Debug)]
pub struct TaskRequires {
    #[knuffel(argument)]
    task: Spanned<String>,

    #[knuffel(property(name = "in"))]
    target: Spanned<String>,
}

impl TaskRequires {
    pub fn parse(
        self,
        workspace_root: &WorkspaceRoot,
    ) -> Result<validated::TaskRequires, TaskValidationError> {
        use chumsky::Parser;
        use target_selector::ParsedSelector;

        let target_span = self.target.span;

        let target = target_selector::parser()
            .parse(self.target.as_str())
            .map_err(|e| {
                let err = e.first().unwrap();
                let err_span = err.span();
                TaskValidationError::MalformedRequire {
                    span: target_span.subspan(err_span.start + 1, err_span.len()),
                    message: err.to_string(),
                }
            })?;

        let (anchor, selection) = match target {
            ParsedSelector::Project(anchor) => (anchor, Selection::Project),
            ParsedSelector::ProjectWithDependencies(anchor) => {
                (anchor, Selection::ProjectWithDependencies)
            }
            ParsedSelector::JustDependencies(anchor) => (anchor, Selection::JustDependencies),
        };

        let anchor_span = {
            let span = anchor.span();
            target_span.subspan(span.start + 1, span.len())
        };

        let target = validated::TargetSelector {
            anchor: anchor
                .validate(workspace_root)
                .map_err(|e| {
                    TaskValidationError::InvalidPaths(vec![ConfigPathValidationError::new(
                        e,
                        anchor_span,
                    )])
                })?
                .with_span(anchor_span),
            selection,
        };

        Ok(validated::TaskRequires {
            task: self.task.into_inner(),
            target,
        })
    }
}

#[derive(knuffel::Decode, Debug)]
pub struct InputBlock {
    #[knuffel(children(name = "path"), unwrap(argument))]
    paths: Vec<Glob>,

    #[knuffel(children(name = "env_var"), unwrap(argument))]
    env_vars: Vec<String>,

    #[knuffel(children(name = "command"), unwrap(argument))]
    commands: Vec<String>,
}

mod target_selector {
    use std::ops::Range;

    use camino::Utf8PathBuf;
    use chumsky::prelude::*;

    use crate::config::{paths::PathError, TargetAnchor, WorkspaceRoot};

    #[derive(Clone, Debug)]
    pub enum ParsedSelector {
        Project(ParsedAnchor),
        ProjectWithDependencies(ParsedAnchor),
        JustDependencies(ParsedAnchor),
    }

    #[derive(Clone, Debug)]
    pub enum ParsedAnchor {
        CurrentProject(Range<usize>),
        ProjectByName(String, Range<usize>),
        ProjectByPath(Utf8PathBuf, Range<usize>),
    }

    impl ParsedAnchor {
        pub fn span(&self) -> Range<usize> {
            match self {
                ParsedAnchor::CurrentProject(span) => span.clone(),
                ParsedAnchor::ProjectByName(_, span) => span.clone(),
                ParsedAnchor::ProjectByPath(_, span) => span.clone(),
            }
        }

        pub fn validate(self, workspace_root: &WorkspaceRoot) -> Result<TargetAnchor, PathError> {
            Ok(match self {
                ParsedAnchor::CurrentProject(_) => TargetAnchor::CurrentProject,
                ParsedAnchor::ProjectByName(name, _) => TargetAnchor::ProjectByName(name),
                ParsedAnchor::ProjectByPath(path, _) => {
                    TargetAnchor::ProjectByPath(workspace_root.subpath(path)?.validate()?)
                }
            })
        }
    }

    pub fn parser() -> impl chumsky::Parser<char, ParsedSelector, Error = Simple<char>> {
        let is_package_char = |c: &char| c.is_alphabetic() || *c == '_' || *c == '-';
        let is_path_char = |_: &char| true;

        let package_name = filter(is_package_char)
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_package_char).repeated())
            .collect::<String>()
            .map_with_span(ParsedAnchor::ProjectByName);

        let package_path = filter(|c| *c == '/')
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_path_char).repeated())
            .collect::<String>()
            .from_str()
            .try_map(|val: Result<Utf8PathBuf, _>, span| {
                val.map_err(|e| Simple::custom(span, format!("Couldn't parse a path: {e}")))
            })
            .map_with_span(ParsedAnchor::ProjectByPath);

        let anchor = choice::<_, Simple<char>>((
            text::keyword("self")
                .to(())
                .map_with_span(|_, span| ParsedAnchor::CurrentProject(span)),
            package_path,
            package_name,
        ));

        choice::<_, Simple<char>>((
            just(['.', '.', '.', '^'])
                .ignore_then(anchor.clone())
                .map(ParsedSelector::JustDependencies),
            just(['.', '.', '.'])
                .ignore_then(anchor.clone())
                .map(ParsedSelector::ProjectWithDependencies),
            anchor.map(ParsedSelector::Project),
        ))
    }

    #[cfg(test)]
    mod tests {

        use assert_matches::assert_matches;

        use super::*;

        #[test]
        fn parsing_selector() {
            assert_matches!(
                parser().parse("self").unwrap(),
                ParsedSelector::Project(ParsedAnchor::CurrentProject(_))
            );
            assert_matches!(
                parser().parse("...self").unwrap(),
                ParsedSelector::ProjectWithDependencies(ParsedAnchor::CurrentProject(_))
            );
            assert_matches!(
                parser().parse("...^self").unwrap(),
                ParsedSelector::JustDependencies(ParsedAnchor::CurrentProject(_))
            );

            assert_matches!(
                parser().parse("some-project-name").unwrap(),
                ParsedSelector::Project(ParsedAnchor::ProjectByName(name, _)) => {
                    assert_eq!(name, "some-project-name");
                }
            );
            assert_matches!(
                parser().parse("...some-project-name").unwrap(),
                ParsedSelector::ProjectWithDependencies(ParsedAnchor::ProjectByName(name, _)) => {
                    assert_eq!(name, "some-project-name");
                }
            );
            assert_matches!(
                parser().parse("...^some-project-name").unwrap(),
                ParsedSelector::JustDependencies(ParsedAnchor::ProjectByName(name, _)) => {
                    assert_eq!(name, "some-project-name");
                }
            );

            assert_matches!(
                parser().parse("/lib/project").unwrap(),
                ParsedSelector::Project(ParsedAnchor::ProjectByPath(name, _)) => {
                    assert_eq!(name, "/lib/project");
                }
            );
            assert_matches!(
                parser().parse(".../lib/project").unwrap(),
                ParsedSelector::ProjectWithDependencies(ParsedAnchor::ProjectByPath(name, _)) => {
                    assert_eq!(name, "/lib/project");
                }
            );
            assert_matches!(
                parser().parse("...^/lib/project").unwrap(),
                ParsedSelector::JustDependencies(ParsedAnchor::ProjectByPath(name, _)) => {
                    assert_eq!(name, "/lib/project");
                }
            );
        }
    }
}
