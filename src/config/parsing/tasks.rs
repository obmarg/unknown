use crate::config::{
    paths::{ConfigPath, ConfigPathValidationError, ValidPath},
    spanned::{SourceSpanExt, Spanned, WithSpan},
    validated, Glob, WorkspaceRoot,
};

use super::CollectResults;

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
    InvalidTasks(#[related] Vec<TaskValidationError>),
    InvalidRequires(#[related] Vec<TaskValidationError>),
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

    #[knuffel(children(name = "require"))]
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
    #[knuffel(property)]
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

        let task_span = self.task.span.clone();

        let target = target_selector::parser()
            .parse(self.task.as_str())
            .map_err(|e| {
                let err = e.first().unwrap();
                let err_span = err.span();
                TaskValidationError::MalformedRequire {
                    span: task_span.subspan(err_span.start, err_span.len()),
                    message: err.to_string(),
                }
            })?;

        let target = match target {
            ParsedSelector::Project(inner, span) => validated::TargetSelector::Project(
                inner
                    .validate(workspace_root)
                    .map_err(|e| {
                        TaskValidationError::InvalidPaths(vec![ConfigPathValidationError::new(
                            e,
                            task_span.subspan(span.start, span.len()),
                        )])
                    })?
                    .with_span(task_span.subspan(span.start, span.len())),
            ),
            ParsedSelector::ProjectWithDependencies(inner, span) => {
                validated::TargetSelector::ProjectWithDependencies(
                    inner
                        .validate(workspace_root)
                        .map_err(|e| {
                            TaskValidationError::InvalidPaths(vec![ConfigPathValidationError::new(
                                e,
                                task_span.subspan(span.start, span.len()),
                            )])
                        })?
                        .with_span(task_span.subspan(span.start, span.len())),
                )
            }
            ParsedSelector::JustDependencies(inner, span) => {
                validated::TargetSelector::JustDependencies(
                    inner
                        .validate(workspace_root)
                        .map_err(|e| {
                            TaskValidationError::InvalidPaths(vec![ConfigPathValidationError::new(
                                e,
                                task_span.subspan(span.start, span.len()),
                            )])
                        })?
                        .with_span(task_span.subspan(span.start, span.len())),
                )
            }
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
    use knuffel::{ast::Literal, decode::Kind, errors::DecodeError, traits::ErrorSpan};

    use crate::config::{paths::PathError, validated, TargetAnchor, TargetSelector, WorkspaceRoot};

    #[derive(Clone, Debug)]
    pub enum ParsedSelector {
        Project(ParsedAnchor, Range<usize>),
        ProjectWithDependencies(ParsedAnchor, Range<usize>),
        JustDependencies(ParsedAnchor, Range<usize>),
    }

    #[derive(Clone, Debug)]
    pub enum ParsedAnchor {
        CurrentProject,
        ProjectByName(String),
        ProjectByPath(Utf8PathBuf),
    }

    impl ParsedAnchor {
        pub fn validate(self, workspace_root: &WorkspaceRoot) -> Result<TargetAnchor, PathError> {
            Ok(match self {
                ParsedAnchor::CurrentProject => TargetAnchor::CurrentProject,
                ParsedAnchor::ProjectByName(name) => TargetAnchor::ProjectByName(name),
                ParsedAnchor::ProjectByPath(path) => {
                    TargetAnchor::ProjectByPath(workspace_root.subpath(path)?.validate()?)
                }
            })
        }
    }

    pub fn parser() -> impl chumsky::Parser<char, ParsedSelector, Error = Simple<char>> {
        let is_package_char = |c: &char| c.is_alphabetic() || *c == '_' || *c == '-';
        let is_path_char = |c: &char| true;

        let package_name = filter(is_package_char)
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_package_char).repeated())
            .collect::<String>()
            .map(|name| ParsedAnchor::ProjectByName(name));

        let package_path = filter(|c| *c == '/')
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_path_char).repeated())
            .collect::<String>()
            .from_str()
            .try_map(|val: Result<Utf8PathBuf, _>, span| {
                val.map_err(|e| Simple::custom(span, format!("Couldn't parse a path: {e}")))
            })
            .map(|name| ParsedAnchor::ProjectByPath(name));

        let anchor = choice::<_, Simple<char>>((
            text::keyword("self").to(ParsedAnchor::CurrentProject),
            package_path,
            package_name,
        ));

        choice::<_, Simple<char>>((
            text::keyword("...^")
                .ignore_then(anchor.clone())
                .map_with_span(ParsedSelector::JustDependencies),
            text::keyword("...")
                .ignore_then(anchor.clone())
                .map_with_span(ParsedSelector::ProjectWithDependencies),
            anchor.map_with_span(ParsedSelector::Project),
        ))
    }
}
