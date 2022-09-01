use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{config::ParsingError, workspace::ProjectRef};

mod cli;
mod config;
mod git;
mod workspace;

// TODO: Consider using camino::Utf8PathBuf everywhere instead...

fn main() -> miette::Result<()> {
    cli::run()
}
