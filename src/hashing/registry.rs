use std::{collections::HashMap, fs::File, sync::Mutex};

use camino::Utf8PathBuf;

use crate::workspace::{TaskRef, Workspace};

use super::{Hash, TaskHashes};

pub struct HashRegistry {
    path: Utf8PathBuf,
    hashes: Mutex<HashMap<TaskRef, TaskHashes>>,
}

impl HashRegistry {
    pub fn for_workspace(workspace: &Workspace) -> Result<Self, HashRegistryLoadError> {
        let mut path = Utf8PathBuf::from(workspace.root_path().clone());
        path.push(".nabs");
        path.push("hashes.json");
        if !path.exists() {
            return Ok(HashRegistry {
                path,
                hashes: Mutex::new(HashMap::new()),
            });
        }

        let contents = serde_json::from_reader::<_, RegistryFileFormat>(File::open(&path)?)?;

        let hashes = match contents {
            #[allow(deprecated)]
            RegistryFileFormat::V1 { .. } => panic!("v1 hashes no longer supported"),
            RegistryFileFormat::V2 { hashes } => hashes
                .into_iter()
                .flat_map(|task_hashes| {
                    let project = workspace.project_at_path(task_hashes.project)?;
                    let task_ref = project
                        .lookup_task(&task_hashes.task, workspace)?
                        .task_ref();
                    Some((task_ref, task_hashes.hahes))
                })
                .collect(),
        };

        Ok(HashRegistry {
            path,
            hashes: Mutex::new(hashes),
        })
    }

    pub fn lookup(&self, task: &TaskRef) -> Option<TaskHashes> {
        let hashes = self.hashes.lock().expect("to be able to lock hashes");
        hashes.get(task).copied()
    }

    pub fn update_input_hash(&self, task: TaskRef, hash: Hash) {
        let mut hashes = self.hashes.lock().expect("to be able to lock hashes");
        let entry = hashes.entry(task).or_insert_with(Default::default);
        entry.inputs = Some(hash);
    }

    pub fn save(self) -> Result<(), HashRegistrySaveError> {
        let hashes = self.hashes.into_inner().expect("Mutex to not be poisoned");
        let contents = RegistryFileFormat::V2 {
            hashes: hashes
                .into_iter()
                .map(|(task_ref, hashes)| V2SerializableHashes {
                    project: task_ref.project().as_str().to_owned(),
                    task: task_ref.task_name().to_owned(),
                    hahes: hashes,
                })
                .collect(),
        };

        let dir_path = self.path.parent().expect("path to have a parent");
        if !dir_path.exists() {
            std::fs::create_dir(dir_path)?;
        }

        serde_json::to_writer(File::create(self.path)?, &contents)?;

        Ok(())
    }
}

#[allow(deprecated)]
mod format {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(tag = "version")]
    pub(super) enum RegistryFileFormat {
        #[deprecated]
        V1 {
            hashes: HashMap<String, TaskHashes>,
        },
        V2 {
            hashes: Vec<V2SerializableHashes>,
        },
    }
}

use format::RegistryFileFormat;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[diagnostic(help(
    "There was a problem loading .nabs/hashes.json.  Try deleting it and trying again"
))]
pub enum HashRegistryLoadError {
    #[error("Couldn't load the hashes file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Couldn't deserialize hashes file: {0}")]
    DeserializeError(#[from] serde_json::Error),
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum HashRegistrySaveError {
    #[error("Couldn't save the hashes file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Couldn't serialize hashes file: {0}")]
    SerializeError(#[from] serde_json::Error),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct V2SerializableHashes {
    project: String,
    task: String,
    hahes: TaskHashes,
}
