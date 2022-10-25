use std::str::FromStr;

use camino::Utf8PathBuf;
use chumsky::prelude::*;

#[derive(Debug)]
pub struct ProjectFilter {
    pub specs: Vec<FilterSpec>,
}

impl ProjectFilter {
    pub fn path(p: Utf8PathBuf) -> Self {
        ProjectFilter {
            specs: vec![FilterSpec {
                matcher: Matcher::Path(p),
                include_dependencies: false,
                include_dependents: false,
            }],
        }
    }
}

#[derive(Debug)]
pub struct FilterSpec {
    pub include_dependents: bool,
    pub include_dependencies: bool,
    pub matcher: Matcher,
}

#[derive(Debug)]
pub enum Matcher {
    Path(Utf8PathBuf),
    Name(String),
}

impl std::fmt::Display for ProjectFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for spec in &self.specs {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{spec}")?;
            first = false;
        }
        Ok(())
    }
}

impl std::fmt::Display for FilterSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.include_dependents {
            write!(f, "...")?;
        }
        match &self.matcher {
            Matcher::Path(_) => todo!(),
            Matcher::Name(n) => write!(f, "{n}")?,
        }
        if self.include_dependencies {
            write!(f, "...")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Couldn't parse a package filter: {errors}")]
pub struct PackageFilterParseErrors {
    errors: String,
}

impl PackageFilterParseErrors {
    fn new(errors: Vec<Simple<char>>) -> Self {
        PackageFilterParseErrors {
            errors: errors
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl FromStr for ProjectFilter {
    type Err = PackageFilterParseErrors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream = chumsky::Stream::from(s);
        parser()
            .parse(stream)
            .map_err(PackageFilterParseErrors::new)
    }
}

fn parser() -> impl chumsky::Parser<char, ProjectFilter, Error = Simple<char>> {
    let is_package_char = |c: &char| c.is_alphabetic() || *c == '_' || *c == '-';

    let package_name = filter(is_package_char)
        .map(Some)
        .chain::<char, Vec<_>, _>(filter(is_package_char).repeated())
        .collect::<String>()
        .map(|name| FilterSpec {
            include_dependents: false,
            include_dependencies: false,
            matcher: Matcher::Name(name),
        });

    package_name
        .separated_by(just(','))
        .allow_trailing()
        .at_least(1)
        .map(|specs| ProjectFilter { specs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::package_name("a-library")]
    #[case::several_package_names("a-library,a-service")]
    fn test_parsing_package_name(#[case] input: &str) {
        assert_eq!(input.parse::<ProjectFilter>().unwrap().to_string(), input);
    }
}
