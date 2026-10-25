//! Adds filesystem context only at the Cargo boundary, outside ABI/source graphs.

use std::{
    fs,
    path::{Path, PathBuf},
};
use super::{BuildError, write_changed};
use std::collections::BTreeMap;
use crate::{RustSourceMap, ResolvedModules};

/// Resolves each original file through its owner, never through the consumer root.
/// Built-in sources already live in the resolved graph; write their exact text
/// into OUT_DIR so editor navigation does not depend on the compiler's checkout.
pub(super) fn resolve_locations(map: &mut RustSourceMap, output: &Path, roots: &BTreeMap<String, PathBuf>, graph: &ResolvedModules) -> Result<(), BuildError> {
    for file in &mut map.sources {
        let source = if crate::is_bundled_module(&file.module) {
            let source = graph
                .module(&file.module)
                .and_then(|module| module.source.sources.iter().find(|source| source.path() == file.path))
                .ok_or_else(|| BuildError::Idl(format!("missing bundled source {}:{}", file.module, file.path)))?;
            let path = output.join("idl-sources").join(&file.path);
            let directory = path.parent().expect("bundled source path has a directory");
            fs::create_dir_all(directory).map_err(|source| BuildError::Io {
                operation: "create bundled source-map directory",
                path: directory.to_owned(),
                source,
            })?;
            write_changed(&path, source.text().as_bytes())?;
            path
        } else {
            roots
                .get(&file.module)
                .ok_or_else(|| BuildError::Idl(format!("missing source-map root for module {:?}", file.module)))?
                .join(&file.path)
        };
        file.location = Some(relative_location(output, &source)?);
    }
    Ok(())
}

/// Resolves an existing file or directory relative to the generated sidecar directory.
/// Different filesystem roots (for example Windows drives) require an absolute path.
pub(super) fn relative_location(output: &Path, source: &Path) -> Result<String, BuildError> {
    let canonical = |path: &Path| {
        fs::canonicalize(path).map_err(|source| BuildError::Io {
            operation: "resolve source-map location",
            path: path.to_owned(),
            source,
        })
    };
    let output = canonical(output)?;
    let source = canonical(source)?;
    let relative = relative_path(&output, &source);
    relative
        .to_str()
        .map(|text| match relative.is_absolute() {
            // Preserve Windows verbatim/UNC prefixes when crossing filesystem roots.
            true => text.to_owned(),
            false => text.replace('\\', "/"),
        })
        .ok_or_else(|| BuildError::Idl("source-map locations must have UTF-8 paths".into()))
}

/// Computes a lexical relative path between already normalized absolute directories.
fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.components().collect::<Vec<_>>();
    let to_parts = to.components().collect::<Vec<_>>();
    let common = from.iter().zip(&to_parts).take_while(|(left, right)| left == right).count();
    if common == 0 {
        return to.to_owned();
    }
    let mut result = PathBuf::new();
    for _ in &from[common..] {
        result.push("..");
    }
    for component in &to_parts[common..] {
        result.push(component.as_os_str());
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source roots resolve from the map location, including paths with spaces and '..'.
    #[test]
    fn emitted_root_finds_authored_files_from_the_map_directory() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("build output/nested");
        let source = root.path().join("contract source/idl");
        fs::create_dir_all(&output).unwrap();
        fs::create_dir_all(&source).unwrap();
        let mapped = relative_location(&output, &source.join(".")).unwrap();
        assert!(!Path::new(&mapped).is_absolute());
        assert_eq!(output.join(mapped).canonicalize().unwrap(), source.canonicalize().unwrap());
        assert_eq!(relative_location(&source, &source).unwrap(), ".");
    }
}
