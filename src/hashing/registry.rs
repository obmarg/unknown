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
        let mut path = Utf8PathBuf::from(workspace.root_path());
        path.push(".nabs");
        path.push("hashes.json");
        if !path.exists() {
            return Ok(HashRegistry {
                path,
                hashes: Mutex::new(HashMap::new()),
            });
        }

        let contents = serde_json::from_reader::<_, RegistryFileFormat>(File::open(&path)?)?;

        let RegistryFileFormat::V1 { hashes } = contents;

        let hashes = hashes
            .into_iter()
            .filter_map(|(task_ref, hashes)| {
                let (project, task) = task_ref.split_once("::")?;

                let task_ref = workspace
                    .lookup_project(&project)?
                    .lookup_task(task)?
                    .task_ref();

                Some((task_ref, hashes))
            })
            .collect();

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
        let contents = RegistryFileFormat::V1 {
            hashes: hashes
                .into_iter()
                .map(|(task_ref, hashes)| {
                    (
                        format!("{}::{}", task_ref.project_name(), task_ref.task_name()),
                        hashes,
                    )
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "version")]
enum RegistryFileFormat {
    V1 { hashes: HashMap<String, TaskHashes> },
}

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
