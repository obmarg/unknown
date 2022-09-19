mod registry;

use std::io::Read;

use camino::Utf8PathBuf;
use rayon::prelude::*;

pub use registry::{HashRegistry, HashRegistryLoadError};

use crate::workspace::{ProjectInfo, TaskInfo};

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

pub fn hash_task_inputs(project: &ProjectInfo, task: &TaskInfo) -> Result<Hash, HashError> {
    // Ok, so for now we just assume the inputs for a task are the files of the project.
    //
    // At a later date we can include whether downstream tasks have been run, then
    // we can build on that to include downstream task otutputs.

    // TODO: Could look into using an ignore based parallel iterator here.
    // Or could maybe use rayon?  Not sure.

    // TODO: Check if files is always sorted.
    // If it's not we'll need to sort it so we get consistent hashes.
    let files = ignore::WalkBuilder::new(project.root.clone())
        .hidden(false)
        .build()
        .filter_map(|f| f.ok())
        .filter(|f| f.path().is_file())
        .map(|f| Utf8PathBuf::try_from(f.into_path()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut file_hashes = Vec::with_capacity(files.len());
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
        .collect_into_vec(&mut file_hashes);

    let mut hasher = blake3::Hasher::new();
    for hash in file_hashes {
        hasher.update(hash.as_bytes());
    }
    let final_hash = hasher.finalize();

    Ok(Hash(*final_hash.as_bytes()))
}
