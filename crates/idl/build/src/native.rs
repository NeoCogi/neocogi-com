//! Standalone Rust description generation and native artifact compilation.
//!
//! Contract crates keep typed bindings in their ordinary Rust `rlib`, while
//! this module emits one metadata-only library for each exact module revision.
//! The small library is shared by every host and plugin that uses that revision,
//! preventing transitive description tables from being copied into each final
//! consumer.

use std::{
    collections::BTreeSet,
    env, fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

use neocogi_com_core::NcGuid;
use neocogi_idl::{DeclarationKind, ResolvedModule, Threading};

use crate::{
    ARTIFACT_SCHEMA, ArtifactRecord, BuildError,
    generate::{native_getter_name, native_link_name},
};

/// Compiles the one canonical description library owned by a module crate.
///
/// The generated source is self-contained and `no_std`: it repeats only the
/// public C layouts needed by the description ABI and therefore needs no Rust
/// package dependency or nested Cargo invocation. Dynamic assemblies receive a
/// shared library; monolith and WebAssembly builds receive a small archive.
pub(crate) fn build(module: &ResolvedModule, dependencies: &[ArtifactRecord], output_dir: &Path) -> Result<ArtifactRecord, BuildError> {
    let target = required_text("TARGET")?;
    let target_os = required_text("CARGO_CFG_TARGET_OS")?;
    // A custom Rust target name is not necessarily installed in the selected
    // sysroot. Assembly tooling supplies the compatible built-in ABI target for
    // this self-contained metadata compilation.
    let native_target = optional_nonempty_text("NEOCOGI_CONTRACT_NATIVE_TARGET")?.unwrap_or_else(|| target.clone());
    let requested_linkage = env::var("NEOCOGI_CONTRACT_LINKAGE").unwrap_or_else(|_| {
        if target_os == "none" || target.starts_with("wasm") {
            "static".into()
        } else {
            "dynamic".into()
        }
    });
    if requested_linkage != "dynamic" && requested_linkage != "static" {
        return Err(BuildError::Idl(format!(
            "NEOCOGI_CONTRACT_LINKAGE must be dynamic or static, found {requested_linkage:?}"
        )));
    }
    validate_dependencies(module, dependencies, &target, &requested_linkage)?;

    let link_name = native_link_name(module);
    let getter = native_getter_name(module);
    let source = generate_descriptor_rust(module, &getter);
    let source_path = output_dir.join(format!("{link_name}.rs"));
    write_changed(&source_path, source.as_bytes())?;

    let (library, linker_files) = if requested_linkage == "dynamic" {
        compile_dynamic(&native_target, &target_os, &link_name, &source_path, dependencies, output_dir)?
    } else {
        compile_static(&native_target, &link_name, &source_path, output_dir)?
    };
    println!("cargo::rustc-link-search=native={}", output_dir.display());
    let cargo_linkage = if requested_linkage == "dynamic" { "dylib" } else { "static" };
    println!("cargo::rustc-link-lib={cargo_linkage}={link_name}");

    Ok(ArtifactRecord {
        schema: ARTIFACT_SCHEMA,
        module: module.source.name.to_string(),
        module_id: module.module_id.to_string(),
        abi_fingerprint: module.fingerprint_hex(),
        target,
        linkage: requested_linkage,
        link_name,
        library,
        linker_files,
        getter,
    })
}

/// Rejects a dependency artifact compiled for another target or linkage model.
fn validate_dependencies(module: &ResolvedModule, dependencies: &[ArtifactRecord], target: &str, requested_linkage: &str) -> Result<(), BuildError> {
    for dependency in dependencies {
        if dependency.target != target {
            return Err(BuildError::Idl(format!(
                "contract dependency {:?} targets {:?}, but module {} targets {target:?}",
                dependency.module, dependency.target, module.source.name
            )));
        }
        if dependency.linkage != requested_linkage {
            return Err(BuildError::Idl(format!(
                "contract dependency {:?} uses {:?} linkage, but module {} uses {requested_linkage:?}",
                dependency.module, dependency.linkage, module.source.name
            )));
        }
    }
    Ok(())
}

/// Generates a self-contained Rust description with exact dependency imports.
///
/// These private types intentionally mirror the public `neocogi-com-core` C
/// layouts. Keeping the native artifact dependency-free avoids exposing Cargo
/// package paths or Rust ABI details through the deployment boundary.
fn generate_descriptor_rust(module: &ResolvedModule, getter: &str) -> String {
    let mut output = String::from(
        "//! Generated immutable contract description. Edit the owning .ncidl source.\n\n\
         #![no_std]\n\n\
         /// Handles an impossible panic path without linking the Rust standard library.\n\
         #[cfg(neocogi_dynamic_descriptor)]\n\
         #[panic_handler]\n\
         fn panic(_information: &core::panic::PanicInfo<'_>) -> ! {\n\
             loop { core::hint::spin_loop(); }\n\
         }\n\n\
         /// ABI version of the generated module-description layout.\n\
         const DESCRIPTION_ABI_VERSION: u32 = 1;\n\n\
         /// C-layout representation of one stable contract identity.\n\
         #[repr(C)]\n\
         struct Guid {\n\
             /// First UUID field in native integer representation.\n\
             data1: u32,\n\
             /// Second UUID field in native integer representation.\n\
             data2: u16,\n\
             /// Third UUID field in native integer representation.\n\
             data3: u16,\n\
             /// Remaining UUID bytes in canonical order.\n\
             data4: [u8; 8],\n\
         }\n\n\
         /// Borrowed immutable UTF-8 owned by this library's static storage.\n\
         #[repr(C)]\n\
         struct StaticUtf8 {\n\
             /// First byte, or a non-dereferenced dangling value when empty.\n\
             data: *const u8,\n\
             /// Number of readable UTF-8 bytes.\n\
             len: u32,\n\
         }\n\n\
         // SAFETY: Every generated view points only to immutable static bytes.\n\
         unsafe impl Sync for StaticUtf8 {}\n\n\
         /// Complete SHA-256 identity of one resolved contract graph.\n\
         #[repr(C)]\n\
         struct Fingerprint {\n\
             /// Digest bytes in canonical display order.\n\
             bytes: [u8; 32],\n\
         }\n\n\
         /// Stable getter signature shared by dependency and root references.\n\
         type GetDescription = unsafe extern \"C\" fn() -> *const ModuleDescription;\n\n\
         /// Exact identity and getter for one direct contract dependency.\n\
         #[repr(C)]\n\
         struct DependencyDescription {\n\
             /// Stable logical dependency identity.\n\
             module_id: Guid,\n\
             /// Exact transitive ABI revision required by this module.\n\
             abi_fingerprint: Fingerprint,\n\
             /// Getter exported by the fingerprint-qualified dependency library.\n\
             get: Option<GetDescription>,\n\
         }\n\n\
         /// Runtime-visible identity of one top-level IDL declaration.\n\
         #[repr(C)]\n\
         struct DeclarationDescription {\n\
             /// Stable declaration identity.\n\
             declaration_id: Guid,\n\
             /// Positive authored declaration version.\n\
             version: u32,\n\
             /// Public declaration-kind ABI tag.\n\
             kind: u32,\n\
             /// Public threading ABI tag or the no-threading sentinel.\n\
             threading: u32,\n\
             /// Exact dotted module and declaration name.\n\
             qualified_name: StaticUtf8,\n\
         }\n\n\
         // SAFETY: Generated rows contain only values and immutable static text.\n\
         unsafe impl Sync for DeclarationDescription {}\n\n\
         /// Canonical runtime description for one exact module revision.\n\
         #[repr(C)]\n\
         struct ModuleDescription {\n\
             /// Version of this C-layout description record.\n\
             description_abi_version: u32,\n\
             /// Stable logical module identity.\n\
             module_id: Guid,\n\
             /// Complete transitive ABI fingerprint.\n\
             abi_fingerprint: Fingerprint,\n\
             /// Exact dotted logical module name.\n\
             module_name: StaticUtf8,\n\
             /// Number of rows addressed by `dependencies`.\n\
             dependency_count: u32,\n\
             /// Canonically ordered direct dependency rows.\n\
             dependencies: *const DependencyDescription,\n\
             /// Number of rows addressed by `declarations`.\n\
             declaration_count: u32,\n\
             /// Source-ordered top-level declaration rows.\n\
             declarations: *const DeclarationDescription,\n\
         }\n\n\
         // SAFETY: The root and all tables it reaches are immutable statics.\n\
         unsafe impl Sync for ModuleDescription {}\n\n",
    );

    for dependency in &module.dependencies {
        let dependency_getter = dependency_getter(dependency);
        output.push_str(&format!(
            "#[link(name = \"{}\")]\nunsafe extern \"C\" {{\n    /// Returns the exact dependency revision named by this import.\n    fn {dependency_getter}() -> *const ModuleDescription;\n}}\n\n",
            native_link_name_for_dependency(dependency)
        ));
    }

    rust_byte_array(&mut output, "MODULE_NAME", module.source.name.to_string().as_bytes());
    for (index, identity) in module.declarations.iter().enumerate() {
        rust_byte_array(&mut output, &format!("DECLARATION_NAME_{index}"), identity.qualified_name.as_bytes());
    }

    output.push_str("\n/// Exact direct dependencies in canonical module-name order.\nstatic DEPENDENCIES: &[DependencyDescription] = &[\n");
    for dependency in &module.dependencies {
        output.push_str(&format!(
            "    DependencyDescription {{ module_id: {}, abi_fingerprint: {}, get: Some({}) }},\n",
            guid_initializer(dependency.module_id),
            fingerprint_initializer(&dependency.abi_fingerprint),
            dependency_getter(dependency)
        ));
    }
    output.push_str("];\n\n/// Public declaration identities available to runtime inspection.\nstatic DECLARATIONS: &[DeclarationDescription] = &[\n");
    for (index, (identity, declaration)) in module.declarations.iter().zip(&module.source.declarations).enumerate() {
        output.push_str(&format!(
            "    DeclarationDescription {{ declaration_id: {}, version: {}, kind: {}, threading: {}, qualified_name: StaticUtf8 {{ data: DECLARATION_NAME_{index}.as_ptr(), len: {} }} }},\n",
            guid_initializer(identity.id),
            identity.version,
            declaration_kind(&declaration.kind),
            declaration_threading(&declaration.kind),
            identity.qualified_name.len()
        ));
    }
    output.push_str("];\n\n/// Complete immutable metadata root for this module revision.\nstatic MODULE: ModuleDescription = ModuleDescription {\n");
    output.push_str("    description_abi_version: DESCRIPTION_ABI_VERSION,\n");
    output.push_str(&format!("    module_id: {},\n", guid_initializer(module.module_id)));
    output.push_str(&format!("    abi_fingerprint: {},\n", fingerprint_initializer(&module.abi_fingerprint)));
    output.push_str(&format!(
        "    module_name: StaticUtf8 {{ data: MODULE_NAME.as_ptr(), len: {} }},\n",
        module.source.name.to_string().len()
    ));
    output.push_str(&format!("    dependency_count: {},\n", module.dependencies.len()));
    output.push_str("    dependencies: DEPENDENCIES.as_ptr(),\n");
    output.push_str(&format!("    declaration_count: {},\n", module.declarations.len()));
    output.push_str("    declarations: DECLARATIONS.as_ptr(),\n};\n\n");
    output.push_str(&format!(
        "/// Returns this library's canonical immutable module description.\n\
         ///\n\
         /// # Safety\n\
         ///\n\
         /// The returned pointer remains valid only while this library is loaded.\n\
         #[unsafe(export_name = \"{getter}\")]\n\
         unsafe extern \"C\" fn get_module_description() -> *const ModuleDescription {{\n\
             &MODULE\n\
         }}\n"
    ));
    output
}

/// Returns the exact getter symbol imported for one dependency revision.
fn dependency_getter(dependency: &neocogi_idl::ResolvedDependency) -> String {
    format!("{}_get_v1", native_link_name_for_dependency(dependency))
}

/// Returns the fingerprint-qualified linker name for one dependency revision.
fn native_link_name_for_dependency(dependency: &neocogi_idl::ResolvedDependency) -> String {
    format!(
        "nc_contract_{}_{}",
        dependency.module_name.replace('.', "_"),
        fingerprint_hex(&dependency.abi_fingerprint)
    )
}

/// Writes one fixed Rust byte array without relying on string escaping.
fn rust_byte_array(output: &mut String, name: &str, bytes: &[u8]) {
    use fmt::Write as _;

    writeln!(output, "/// UTF-8 bytes retained in this library for `{name}`.").expect("writing to String cannot fail");
    write!(output, "static {name}: [u8; {}] = [", bytes.len()).expect("writing to String cannot fail");
    for (index, byte) in bytes.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        write!(output, "{byte}").expect("writing to String cannot fail");
    }
    output.push_str("];\n");
}

/// Renders a Rust aggregate expression for one ABI GUID.
fn guid_initializer(guid: NcGuid) -> String {
    let (data1, data2, data3, data4) = guid.fields();
    let bytes = data4.iter().map(u8::to_string).collect::<Vec<_>>().join(", ");
    format!("Guid {{ data1: {data1}, data2: {data2}, data3: {data3}, data4: [{bytes}] }}")
}

/// Renders one fingerprint as a Rust fixed-array expression.
fn fingerprint_initializer(fingerprint: &[u8; 32]) -> String {
    let bytes = fingerprint.iter().map(u8::to_string).collect::<Vec<_>>().join(", ");
    format!("Fingerprint {{ bytes: [{bytes}] }}")
}

/// Renders one fingerprint as lowercase hexadecimal text.
fn fingerprint_hex(fingerprint: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in fingerprint {
        use fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

/// Maps a resolved declaration variant to the public C ABI tag.
fn declaration_kind(kind: &DeclarationKind) -> u32 {
    match kind {
        DeclarationKind::Struct(_) => 1,
        DeclarationKind::Enum(_) => 2,
        DeclarationKind::Interface(_) | DeclarationKind::Event(_) => 3,
        DeclarationKind::Class(_) => 4,
        DeclarationKind::Factory(_) => 5,
        DeclarationKind::BitEnum(_) => 6,
    }
}

/// Maps object capabilities to ABI values and marks value/interface rows absent.
fn declaration_threading(kind: &DeclarationKind) -> u32 {
    match kind {
        DeclarationKind::Class(class) => threading_value(class.threading),
        DeclarationKind::Factory(factory) => threading_value(factory.threading),
        DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_) | DeclarationKind::Interface(_) | DeclarationKind::Event(_) => {
            u32::MAX
        }
    }
}

/// Converts one resolved threading capability to its stable raw ABI value.
fn threading_value(threading: Threading) -> u32 {
    match threading {
        Threading::Local => 0,
        Threading::Shared => 1,
    }
}

/// Builds a shared library and returns any platform linker companion files.
fn compile_dynamic(
    target: &str,
    target_os: &str,
    link_name: &str,
    source: &Path,
    dependencies: &[ArtifactRecord],
    output_dir: &Path,
) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
    let library = dynamic_library_path(target_os, link_name, output_dir);
    let mut command = rustc_command(target, link_name, source)?;
    command
        .args(["--crate-type", "cdylib", "--cfg", "neocogi_dynamic_descriptor"])
        // Only the fingerprinted getter is a supported diagnostic symbol. Strip
        // private compiler symbols in every profile to keep metadata DSOs small.
        .arg("-C")
        .arg("strip=symbols")
        .arg("-o")
        .arg(&library);

    let mut search_paths = BTreeSet::new();
    for dependency in dependencies {
        let directory = dependency
            .library
            .parent()
            .ok_or_else(|| BuildError::Idl(format!("contract artifact {} has no directory", dependency.library.display())))?;
        if search_paths.insert(directory) {
            command.arg("-L").arg(format!("native={}", directory.display()));
        }
    }
    if target_os == "linux" {
        command.arg("-C").arg(format!("link-arg=-Wl,-soname,lib{link_name}.so"));
        command.arg("-C").arg("link-arg=-Wl,-rpath,$ORIGIN");
    } else if target_os == "macos" {
        command.arg("-C").arg(format!("link-arg=-Wl,-install_name,@rpath/lib{link_name}.dylib"));
        command.arg("-C").arg("link-arg=-Wl,-rpath,@loader_path");
    }
    run(&mut command, "compile dynamic Rust contract description")?;

    let linker_files = if target.contains("msvc") {
        vec![library.with_extension("dll.lib")]
    } else if target_os == "windows" {
        vec![output_dir.join(format!("lib{link_name}.dll.a"))]
    } else {
        Vec::new()
    };
    for linker_file in &linker_files {
        if !linker_file.is_file() {
            return Err(BuildError::Idl(format!(
                "Rust compiler did not produce expected contract import library {}",
                linker_file.display()
            )));
        }
    }
    Ok((library, linker_files))
}

/// Builds one object with rustc, then archives it for a static final image.
fn compile_static(target: &str, link_name: &str, source: &Path, output_dir: &Path) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
    let object_extension = if target.contains("msvc") { "obj" } else { "o" };
    let object = output_dir.join(format!("{link_name}.{object_extension}"));
    let mut command = rustc_command(target, link_name, source)?;
    command.args(["--crate-type", "lib", "--emit", "obj"]).arg("-o").arg(&object);
    run(&mut command, "compile static Rust contract description")?;

    // `cc` provides the target-specific archiver selection only. No generated C
    // source or C compiler participates in this path.
    cc::Build::new()
        .target(target)
        .out_dir(output_dir)
        .cargo_metadata(false)
        .object(&object)
        .compile(link_name);
    let library = if target.contains("msvc") {
        output_dir.join(format!("{link_name}.lib"))
    } else {
        output_dir.join(format!("lib{link_name}.a"))
    };
    Ok((library.clone(), vec![library]))
}

/// Creates the exact rustc command used for a metadata-only native artifact.
fn rustc_command(target: &str, crate_name: &str, source: &Path) -> Result<Command, BuildError> {
    let compiler = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut command = Command::new(compiler);
    command
        .args(["--edition", "2024", "--crate-name", crate_name, "--target", target])
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg(format!("opt-level={}", required_text("OPT_LEVEL")?))
        .arg(source);
    Ok(command)
}

/// Chooses the conventional loadable filename for one target operating system.
fn dynamic_library_path(target_os: &str, link_name: &str, output_dir: &Path) -> PathBuf {
    match target_os {
        "windows" => output_dir.join(format!("{link_name}.dll")),
        "macos" => output_dir.join(format!("lib{link_name}.dylib")),
        _ => output_dir.join(format!("lib{link_name}.so")),
    }
}

/// Runs one compiler command and preserves its complete failure output.
fn run(command: &mut Command, operation: &'static str) -> Result<(), BuildError> {
    let output = command.output().map_err(|source| BuildError::Io {
        operation,
        path: PathBuf::from(command.get_program()),
        source,
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(BuildError::Command {
        operation,
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Reads one required UTF-8 Cargo build variable.
fn required_text(variable: &'static str) -> Result<String, BuildError> {
    env::var(variable).map_err(|_| BuildError::MissingEnvironment(variable))
}

/// Reads an optional UTF-8 target override and rejects an ambiguous empty value.
fn optional_nonempty_text(variable: &'static str) -> Result<Option<String>, BuildError> {
    match env::var(variable) {
        Ok(value) if value.trim().is_empty() => Err(BuildError::Idl(format!("{variable} must not be empty"))),
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(BuildError::Idl(format!("{variable} must contain UTF-8 text"))),
    }
}

/// Replaces generated Rust source only when its bytes changed.
fn write_changed(path: &Path, contents: &[u8]) -> Result<(), BuildError> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|source| BuildError::Io {
        operation: "write generated native Rust source",
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    //! Description-generation tests independent of a platform linker.

    use std::path::Path;

    use neocogi_idl::{ModuleSource, parse_module_parts, resolve_modules};

    use super::{generate_descriptor_rust, native_getter_name};

    /// Confirms source-part permutations produce byte-identical standalone Rust.
    #[test]
    fn multipart_permutations_generate_identical_native_descriptions() {
        let first = ModuleSource {
            path: Path::new("example/module/values.ncidl"),
            text: "#[version(1)] struct Value { raw: u32 }",
        };
        let second = ModuleSource {
            path: Path::new("example/module/service.ncidl"),
            text: "#[version(1)] trait IService { fn value(&self) -> Result<Value>; }",
        };
        let forward = resolve_modules(vec![parse_module_parts(Path::new("example/module"), &[first, second]).unwrap()]).unwrap();
        let reverse = resolve_modules(vec![parse_module_parts(Path::new("example/module"), &[second, first]).unwrap()]).unwrap();
        let forward = forward.module("example.module").unwrap();
        let reverse = reverse.module("example.module").unwrap();

        assert_eq!(
            generate_descriptor_rust(forward, &native_getter_name(forward)),
            generate_descriptor_rust(reverse, &native_getter_name(reverse))
        );
    }
}
