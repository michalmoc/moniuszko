use std::path::{Path, PathBuf, absolute};

pub fn absolutize(base_dir: PathBuf, path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(absolute(path.to_owned())?)
    } else {
        Ok(absolute(base_dir.join(path))?)
    }
}
