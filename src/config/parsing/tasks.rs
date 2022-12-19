use validated::{SpecificProjectSelector, TargetSelector};

use crate::config::{
    paths::{ConfigPath, ConfigPathValidationError},
    spanned::{SourceSpanExt, Spanned, WithSpan},
    validated, Glob, WorkspaceRoot,
};

#[derive(knuffel::Decode, Debug, Default)]
pub struct TaskBlock {
    #[knuffel(children(name = "import"), unwrap(argument))]
    pub(in crate::config) imports: Vec<ConfigPath>,

    #[knuffel(children(name = "task"))]
    pub(in crate::config) tasks: Vec<TaskDefinition>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum TaskValidationError {
    #[error("Invalid paths in a task file")]
    InvalidPaths(#[related] Vec<ConfigPathValidationError>),
    #[error("Error parsing a task require target")]
    MalformedRequire {
        #[label = "{message}"]
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
    target: Option<Spanned<String>>,
}

impl TaskRequires {
    pub fn parse(
        self,
        workspace_root: &WorkspaceRoot,
    ) -> Result<validated::TaskRequires, TaskValidationError> {
        use chumsky::Parser;
        use target_selector::ParsedSelector;

        let task_span = self.task.span;
        let task = self.task.into_inner().with_span(task_span);

        if self.target.is_none() {
            return Ok(validated::TaskRequires { task, target: None });
        }

        let target = self.target.unwrap();
        let target_span = target.span;

        let target = target_selector::parser()
            .parse(target.as_str())
            .map_err(|e| {
                // TODO: The errors this produces are pretty awful.  Might need to look into
                // another parser at some point.
                let err = e.first().unwrap();
                let err_span = err.span();
                TaskValidationError::MalformedRequire {
                    span: target_span.subspan(err_span.start + 1, err_span.len()),
                    message: err.to_string(),
                }
            })?;

        let target = match target {
            ParsedSelector::CurrentProject => TargetSelector::CurrentProject,
            ParsedSelector::DependenciesOfCurrent => TargetSelector::DependenciesOfCurrent,
            ParsedSelector::ProjectByName(name, span) => TargetSelector::SpecificDependency(
                SpecificProjectSelector::ByName(name)
                    .with_span(target_span.subspan(span.start + 1, span.len())),
            ),
            ParsedSelector::ProjectByPath(path, span) => {
                let span = target_span.subspan(span.start + 1, span.len());
                TargetSelector::SpecificDependency(
                    SpecificProjectSelector::ByPath(
                        workspace_root
                            .subpath(path)
                            .and_then(|p| p.validate())
                            .map_err(|e| {
                                TaskValidationError::InvalidPaths(vec![
                                    ConfigPathValidationError::new(e, span),
                                ])
                            })?,
                    )
                    .with_span(span),
                )
            }
        };

        Ok(validated::TaskRequires {
            task,
            target: Some(target.with_span(target_span)),
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



    #[derive(Clone, Debug)]
    pub enum ParsedSelector {
        CurrentProject,
        DependenciesOfCurrent,
        ProjectByName(String, Range<usize>),
        ProjectByPath(Utf8PathBuf, Range<usize>),
    }

    pub fn parser() -> impl chumsky::Parser<char, ParsedSelector, Error = Simple<char>> {
        let is_package_char = |c: &char| c.is_alphabetic() || *c == '_' || *c == '-';
        let is_path_char = |_: &char| true;

        let current_project = text::keyword("self").to(ParsedSelector::CurrentProject);
        let dependencies = just("^self").to(ParsedSelector::DependenciesOfCurrent);

        let package_name = filter(is_package_char)
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_package_char).repeated())
            .collect::<String>()
            .map_with_span(ParsedSelector::ProjectByName);

        let package_path = filter(|c| *c == '/')
            .map(Some)
            .chain::<char, Vec<_>, _>(filter(is_path_char).repeated())
            .collect::<String>()
            .from_str()
            .try_map(|val: Result<Utf8PathBuf, _>, span| {
                val.map_err(|e| Simple::custom(span, format!("Couldn't parse a path: {e}")))
            })
            .map_with_span(ParsedSelector::ProjectByPath);

        choice::<_, Simple<char>>((current_project, dependencies, package_path, package_name))
    }

    #[cfg(test)]
    mod tests {

        use assert_matches::assert_matches;

        use super::*;

        #[test]
        fn parsing_selector() {
            assert_matches!(
                parser().parse("self").unwrap(),
                ParsedSelector::CurrentProject
            );

            assert_matches!(
                parser().parse("^self").unwrap(),
                ParsedSelector::DependenciesOfCurrent
            );

            assert_matches!(
                parser().parse("some-project-name").unwrap(),
                ParsedSelector::ProjectByName(name, _) => {
                    assert_eq!(name, "some-project-name");
                }
            );

            assert_matches!(
                parser().parse("/lib/project").unwrap(),
                ParsedSelector::ProjectByPath(name, _) => {
                    assert_eq!(name, "/lib/project");
                }
            );
        }
    }
}
