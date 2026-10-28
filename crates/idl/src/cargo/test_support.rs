//! Recursive fixture snapshots shared by Cargo failure-boundary tests.

use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    fmt, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

/// Distinguishes empty directories from files so creation is observable too.
#[derive(Eq, PartialEq)]
pub(crate) enum SnapshotEntry {
    /// An existing directory, including an otherwise empty navigation directory.
    Directory,
    /// Exact bytes of a regular fixture file.
    File(Vec<u8>),
}

impl fmt::Debug for SnapshotEntry {
    /// Keeps failed comparisons readable without dumping entire generated files.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory => formatter.write_str("Directory"),
            Self::File(bytes) => {
                let mut hash = DefaultHasher::new();
                bytes.hash(&mut hash);
                formatter
                    .debug_struct("File")
                    .field("bytes", &bytes.len())
                    .field("diagnostic_hash", &hash.finish())
                    .finish()
            }
        }
    }
}

/// Captures the complete tree; absence differs from an existing empty directory.
pub(crate) fn snapshot(directory: &Path) -> Option<BTreeMap<PathBuf, SnapshotEntry>> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, SnapshotEntry>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                entries.insert(relative, SnapshotEntry::Directory);
                visit(root, &path, entries);
            } else {
                assert!(entry.file_type().unwrap().is_file(), "fixture entries must be regular files");
                entries.insert(relative, SnapshotEntry::File(fs::read(path).unwrap()));
            }
        }
    }
    if !directory.exists() {
        return None;
    }
    let mut entries = BTreeMap::new();
    visit(directory, directory, &mut entries);
    Some(entries)
}

/// Makes preservation include an unrelated nested user-owned file.
pub(crate) fn seed_unrelated(output: &Path) {
    fs::create_dir_all(output.join("unrelated/nested")).unwrap();
    fs::write(output.join("unrelated/nested/sentinel"), "preserve this file").unwrap();
}
