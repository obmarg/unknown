mod registry;

#[cfg(test)]
mod tests;

use std::io::Read;

use camino::Utf8PathBuf;
use globset::{Glob, GlobSetBuilder};
use rayon::prelude::*;

pub use registry::{HashRegistry, HashRegistryLoadError};

use crate::{
    config::NormalisedPath,
    workspace::{ProjectInfo, TaskInfo},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Default)]
pub struct TaskHashes {
    pub inputs: Option<Hash>,
    pub ouptuts: Option<Hash>,
}

// TODO: probably want a better serialization strategy
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct Hash([u8; 32]);

#[derive(thiserror::Error, Debug)]
pub enum HashError {
    #[error("Uncountered a path that wasn't UTF8: {0}")]
    InvalidPathFound(#[from] camino::FromPathBufError),
}

pub fn hash_task_inputs(project: &ProjectInfo, task: &TaskInfo) -> Result<Option<Hash>, HashError> {
    if task.inputs.is_empty() {
        return Ok(None);
    }

    let mut hashes = Vec::with_capacity(task.inputs.len());
    hash_file_inputs(&project.root, &task.inputs.paths, &mut hashes)?;
    hash_env_vars(project, task, &mut hashes)?;
    hash_commands(project, task, &mut hashes)?;
    // TODO: also need to hash the task/project itself somehow...

    let mut hasher = blake3::Hasher::new();
    for hash in hashes {
        hasher.update(hash.as_bytes());
    }
    let final_hash = hasher.finalize();

    Ok(Some(Hash(*final_hash.as_bytes())))
}

fn hash_file_inputs(
    project_root: &NormalisedPath,
    globs: &[Glob],
    hashes: &mut Vec<blake3::Hash>,
) -> Result<(), HashError> {
    if globs.is_empty() {
        return Ok(());
    }

    // TODO: Could look into using an ignore based parallel iterator here.
    // Or could maybe use rayon?  Not sure.

    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        builder.add(glob.clone());
    }
    let globset = builder.build().expect("the globset build to succeed");

    // TODO: Check if files is always sorted.
    // If it's not we'll need to sort it so we get consistent hashes.
    let files = ignore::WalkBuilder::new(project_root.full_path())
        .hidden(false)
        .build()
        .filter_map(|f| f.ok())
        .filter(|entry| globset.is_match(entry.path()))
        .filter(|f| f.path().is_file())
        .map(|f| Utf8PathBuf::try_from(f.into_path()))
        .collect::<Result<Vec<_>, _>>()?;

    files
        .into_par_iter()
        .map(|path| {
            let mut buffer = vec![0; 2048];
            let mut hasher = blake3::Hasher::new();
            let mut file = std::fs::File::open(path).expect("to be able to open file");
            loop {
                let bytes_read = file.read(&mut buffer).expect("to be able to read file");
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[0..bytes_read]);
            }
            hasher.finalize()
        })
        .collect_into_vec(hashes);

    Ok(())
}

pub fn hash_env_vars(
    _project: &ProjectInfo,
    task: &TaskInfo,
    #[allow(clippy::ptr_arg)] _hashes: &mut Vec<blake3::Hash>,
) -> Result<(), HashError> {
    if !task.inputs.env_vars.is_empty() {
        todo!("Need to implement env var hashing")
    }
    Ok(())
}

pub fn hash_commands(
    _project: &ProjectInfo,
    task: &TaskInfo,
    #[allow(clippy::ptr_arg)] _hashes: &mut Vec<blake3::Hash>,
) -> Result<(), HashError> {
    if !task.inputs.commands.is_empty() {
        todo!("Need to implement command hashing")
    }
    Ok(())
}
