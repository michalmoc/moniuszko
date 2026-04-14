use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Deserialize, Serialize)]
struct DatabaseEntry {
    hash: u64,
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
    pub moved: HashSet<(PathBuf, PathBuf)>,
}

impl FilesDatabase {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn scan(&mut self, path: &Path) -> ScanResult {
        let mut result = ScanResult::default();

        let mut found = HashSet::new();
        let mut missing = HashMap::new();

        let mut buffer = Vec::new();

        for (file, entry) in &mut self.entries {
            if file.is_file() {
                found.insert(file.clone());

                let modified_time = fs::metadata(file).unwrap().modified().unwrap();

                if modified_time == entry.last_modified {
                    result.unchanged.insert(file.clone());
                } else {
                    result.modified.insert(file.clone());
                    entry.last_modified = modified_time;

                    let mut content = File::open(file).unwrap();
                    buffer.clear();
                    let _ = content.read_to_end(&mut buffer);
                    entry.hash = seahash::hash(&buffer);
                }
            } else {
                missing.insert(entry.hash.clone(), file.clone());
            }
        }

        for file in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if found.contains(file.path()) {
                continue;
            } else {
                let mut content = File::open(file.path()).unwrap();
                buffer.clear();
                let _ = content.read_to_end(&mut buffer);

                let hash = seahash::hash(&buffer);

                if let Some(old_path) = missing.get(&hash).cloned() {
                    result
                        .moved
                        .insert((old_path.clone(), file.clone().into_path()));
                    missing.remove(&hash);
                    let entry = self.entries.remove(&old_path).unwrap();
                    self.entries.insert(file.into_path(), entry);
                } else {
                    let path = file.into_path();
                    let modified = fs::metadata(&path).unwrap().modified().unwrap();

                    result.new.insert(path.clone());
                    self.entries.insert(
                        path,
                        DatabaseEntry {
                            hash,
                            last_modified: modified,
                        },
                    );
                }
            }
        }

        for (_, file) in missing {
            result.deleted.insert(file.clone());
            self.entries.remove(&file);
        }

        result
    }
}
