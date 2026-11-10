//! Decides where PortWatch keeps its data.
//!
//! Normal install: the per-user app data folder chosen by Tauri
//! (`%APPDATA%\io.github.sam3360.portwatch`).
//! Portable mode: if a file named `portwatch.portable` sits next to the
//! executable, a `data` folder next to the executable is used instead so
//! nothing is written elsewhere on the machine.

use std::path::{Path, PathBuf};

pub const PORTABLE_MARKER: &str = "portwatch.portable";

pub fn resolve_data_dir(default_dir: PathBuf, exe_dir: Option<&Path>) -> (PathBuf, bool) {
    if let Some(dir) = exe_dir {
        if dir.join(PORTABLE_MARKER).is_file() {
            return (dir.join("data"), true);
        }
    }
    (default_dir, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_default_dir_without_marker() {
        let tmp = tempfile::tempdir().unwrap();
        let (dir, portable) = resolve_data_dir(PathBuf::from("/appdata"), Some(tmp.path()));
        assert_eq!(dir, PathBuf::from("/appdata"));
        assert!(!portable);
    }

    #[test]
    fn portable_marker_switches_to_local_data_folder() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(PORTABLE_MARKER), "").unwrap();
        let (dir, portable) = resolve_data_dir(PathBuf::from("/appdata"), Some(tmp.path()));
        assert_eq!(dir, tmp.path().join("data"));
        assert!(portable);
    }

    #[test]
    fn unknown_exe_dir_falls_back_to_default() {
        let (dir, portable) = resolve_data_dir(PathBuf::from("/appdata"), None);
        assert_eq!(dir, PathBuf::from("/appdata"));
        assert!(!portable);
    }
}
