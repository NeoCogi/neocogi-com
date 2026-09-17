//! Canonical filesystem loading for single-file and multipart IDL modules.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts};
use serde::{Deserialize, Serialize};

use crate::cargo::BuildError;

/// Owned source text for exactly one logical IDL module.
///
/// The representation stays private so callers depend on module semantics rather
/// than the JSON graph layout used between Cargo build scripts. Paths exposed by
/// the accessors are normalized paths relative to the configured IDL root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleSources {
    /// File or directory path whose segments determine the logical module name.
    module_path: String,
    /// Source parts in deterministic path order, with exact authored UTF-8 text.
    sources: BTreeMap<String, String>,
}

impl ModuleSources {
    /// Loads a module by inspecting whether its selected path is a file or directory.
    ///
    /// This is the shared entry point for tools that obtain the source path from
    /// package metadata. A file is one legacy module source; a directory owns all
    /// regular `.ncidl` parts recursively below it.
    pub fn load(idl_root: &Path, module_path: &Path) -> Result<Self, BuildError> {
        let source_path = idl_root.join(module_path);
        let metadata = fs::metadata(&source_path).map_err(|source| BuildError::Io {
            operation: "inspect IDL module source",
            path: source_path.clone(),
            source,
        })?;

        // Follow the selected path's filesystem kind once, then delegate to the
        // same explicit loaders used by contract build scripts.
        if metadata.is_file() {
            Self::load_file(idl_root, module_path)
        } else if metadata.is_dir() {
            Self::load_directory(idl_root, module_path)
        } else {
            Err(BuildError::Idl(format!(
                "IDL module source is neither a regular file nor a directory: {}",
                source_path.display()
            )))
        }
    }

    /// Returns the normalized source path that determines module identity.
    pub fn module_path(&self) -> &Path {
        Path::new(&self.module_path)
    }

    /// Borrows source parts in deterministic normalized path order.
    ///
    /// Part paths are diagnostic identities only; declaration ordering and ABI
    /// identity remain the responsibility of the pure IDL parser.
    pub fn parts(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.sources.iter().map(|(path, source)| (Path::new(path), source.as_str()))
    }

    /// Parses the loaded text using file or directory module identity semantics.
    pub fn parse(&self) -> Result<neocogi_idl::Module, BuildError> {
        let module_path = self.module_path();
        if module_path.extension().and_then(|extension| extension.to_str()) == Some("ncidl") {
            if self.sources.len() != 1 {
                return Err(BuildError::Idl(format!(
                    "single-file IDL module {:?} contains {} source records",
                    self.module_path,
                    self.sources.len()
                )));
            }
            // A file module has exactly one source record by construction and by
            // validation of dependency graph records above.
            let source = self.sources.values().next().expect("single-file source count was checked");
            return parse_module(module_path, source).map_err(|source| BuildError::Idl(source.to_string()));
        }

        // Borrow the owned paths and text only for the duration of parsing. The
        // parser canonicalizes portable path order independently as a safeguard.
        let parts = self
            .sources
            .iter()
            .map(|(path, source)| ModuleSource { path: Path::new(path), text: source })
            .collect::<Vec<_>>();
        parse_module_parts(module_path, &parts).map_err(|source| BuildError::Idl(source.to_string()))
    }

    /// Loads one explicitly selected legacy source file.
    pub(crate) fn load_file(idl_root: &Path, relative_path: &Path) -> Result<Self, BuildError> {
        let source_path = idl_root.join(relative_path);
        let relative_text = path_text(relative_path)?;
        let source = read_source(&source_path)?;
        Ok(Self {
            module_path: relative_text.clone(),
            sources: BTreeMap::from([(relative_text, source)]),
        })
    }

    /// Loads every regular `.ncidl` part below one explicitly selected directory.
    pub(crate) fn load_directory(idl_root: &Path, relative_directory: &Path) -> Result<Self, BuildError> {
        let module_directory = idl_root.join(relative_directory);
        let mut paths = Vec::new();
        discover_parts(&module_directory, &mut paths)?;
        paths.sort();

        // Store paths relative to the configured root so diagnostics and graph
        // exchange do not depend on a producer's absolute checkout location.
        let mut sources = BTreeMap::new();
        for source_path in paths {
            let relative_path = source_path.strip_prefix(idl_root).map_err(|_| {
                BuildError::Idl(format!(
                    "discovered IDL source {} outside configured root {}",
                    source_path.display(),
                    idl_root.display()
                ))
            })?;
            sources.insert(path_text(relative_path)?, read_source(&source_path)?);
        }
        Ok(Self {
            module_path: path_text(relative_directory)?,
            sources,
        })
    }
}

/// Recursively discovers regular `.ncidl` parts without assigning order meaning.
fn discover_parts(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    let entries = fs::read_dir(directory).map_err(|source| BuildError::Io {
        operation: "read IDL module directory",
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| BuildError::Io {
            operation: "read IDL module directory entry",
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| BuildError::Io {
            operation: "inspect IDL module directory entry",
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            // Nested directories organize parts of this selected module. Selecting
            // a file module instead never scans a same-named child directory.
            discover_parts(&path, output)?;
        } else if file_type.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("ncidl") {
            output.push(path);
        }
    }
    Ok(())
}

/// Reads one UTF-8 IDL part with its concrete filesystem path in failures.
fn read_source(path: &Path) -> Result<String, BuildError> {
    fs::read_to_string(path).map_err(|source| BuildError::Io {
        operation: "read IDL source",
        path: path.to_owned(),
        source,
    })
}

/// Converts a normalized source path into portable metadata text.
fn path_text(path: &Path) -> Result<String, BuildError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| BuildError::Idl(format!("IDL source path {} is not UTF-8", path.display())))
}

#[cfg(test)]
mod tests {
    //! Filesystem-focused coverage for the source ownership boundary.

    use std::{fs, path::Path};

    use tempfile::tempdir;

    use super::ModuleSources;

    /// Confirms recursive discovery ignores unrelated files and preserves a stable
    /// path order while treating nested directories as organizational source parts.
    #[test]
    fn loads_directory_parts_as_one_module() {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("idl");
        let module = root.join("neocogi/ui");
        let nested = module.join("input");
        fs::create_dir_all(&nested).unwrap();
        fs::write(module.join("widgets.ncidl"), "#[version(1)] interface IWidget {}").unwrap();
        fs::write(nested.join("geometry.ncidl"), "#[version(1)] struct Size { width: u32 }").unwrap();
        fs::write(module.join("README.md"), "not IDL").unwrap();

        let sources = ModuleSources::load(&root, Path::new("neocogi/ui")).unwrap();
        let parsed = sources.parse().unwrap();

        assert_eq!(sources.module_path(), Path::new("neocogi/ui"));
        assert_eq!(
            sources.parts().map(|(path, _)| path).collect::<Vec<_>>(),
            [Path::new("neocogi/ui/input/geometry.ncidl"), Path::new("neocogi/ui/widgets.ncidl")]
        );
        assert_eq!(parsed.name.to_string(), "neocogi.ui");
        assert_eq!(
            parsed.declarations.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            ["IWidget", "Size"]
        );
    }

    /// Confirms a selected file module does not absorb sources belonging to a
    /// potential child module in the adjacent same-named directory.
    #[test]
    fn file_module_excludes_child_module_sources() {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("idl");
        fs::create_dir_all(root.join("neocogi/ui")).unwrap();
        fs::write(root.join("neocogi/ui.ncidl"), "#[version(1)] struct Parent { value: u32 }").unwrap();
        fs::write(root.join("neocogi/ui/child.ncidl"), "#[version(1)] struct Child { value: u32 }").unwrap();

        let sources = ModuleSources::load(&root, Path::new("neocogi/ui.ncidl")).unwrap();
        let parsed = sources.parse().unwrap();

        assert_eq!(sources.parts().count(), 1);
        assert_eq!(parsed.name.to_string(), "neocogi.ui");
        assert_eq!(parsed.declarations[0].name, "Parent");
    }
}
