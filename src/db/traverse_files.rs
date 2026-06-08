use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Deserialize, Serialize)]
struct DatabaseEntry {
    last_modified: SystemTime,
}

#[derive(Deserialize, Serialize, Default)]
pub struct FilesDatabase {
    entries: HashMap<PathBuf, DatabaseEntry>,
}

#[derive(Default)]
pub struct ScanResult {
    pub new: HashSet<PathBuf>,
    pub unchanged: HashSet<PathBuf>,
    pub modified: HashSet<PathBuf>,
    pub deleted: HashSet<PathBuf>,
}

impl FilesDatabase {
    pub fn scan(&mut self, path: &Path) -> ScanResult {
        let mut result = ScanResult::default();

        let mut to_remove = Vec::new();
        for (file, entry) in &mut self.entries {
            if file.is_file() {
                let modified_time = fs::metadata(file).unwrap().modified().unwrap();

                if modified_time == entry.last_modified {
                    result.unchanged.insert(file.clone());
                } else {
                    result.modified.insert(file.clone());
                    entry.last_modified = modified_time;
                }
            } else {
                to_remove.push(file.clone());
                result.deleted.insert(file.clone());
            }
        }

        for file in to_remove {
            self.entries.remove(&file);
        }

        for file in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if self.entries.contains_key(file.path()) {
                continue;
            } else {
                let path = file.into_path();
                let modified = fs::metadata(&path).unwrap().modified().unwrap();

                result.new.insert(path.clone());
                self.entries.insert(
                    path,
                    DatabaseEntry {
                        last_modified: modified,
                    },
                );
            }
        }

        result
    }
}
