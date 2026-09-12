//! Native C description generation and platform-specific library compilation.

use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

use neocogi_com_core::NcGuid;
use neocogi_idl::{DeclarationKind, ResolvedModule, Threading};

use crate::{
    ArtifactRecord, BuildError,
    generate::{native_getter_name, native_link_name},
};

/// Compiles the one canonical native description owned by a module crate.
pub(crate) fn build(module: &ResolvedModule, source: &str, dependencies: &[ArtifactRecord], output_dir: &Path) -> Result<ArtifactRecord, BuildError> {
    let target = required_text("TARGET")?;
    let target_os = required_text("CARGO_CFG_TARGET_OS")?;
    // Cargo's Rust target identity may name a custom JSON target that `cc` cannot
    // interpret as a compiler triple. Assembly tooling supplies the underlying C
    // ABI target explicitly; direct Cargo builds retain their ordinary Rust target.
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

    let link_name = native_link_name(module);
    let getter = native_getter_name(module);
    let c_source = generate_c(module, source, &getter);
    let c_path = output_dir.join(format!("{link_name}.c"));
    write_changed(&c_path, c_source.as_bytes())?;

    let (library, linker_files) = if requested_linkage == "dynamic" {
        compile_dynamic(&native_target, &target_os, &link_name, &c_path, dependencies, output_dir)?
    } else {
        compile_static(&native_target, &link_name, &c_path, output_dir)?
    };
    println!("cargo::rustc-link-search=native={}", output_dir.display());
    let cargo_linkage = if requested_linkage == "dynamic" { "dylib" } else { "static" };
    println!("cargo::rustc-link-lib={cargo_linkage}={link_name}");

    Ok(ArtifactRecord {
        schema: 1,
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

/// Generates a self-contained C description with exact dependency imports.
fn generate_c(module: &ResolvedModule, source: &str, getter: &str) -> String {
    let mut output = String::new();
    output.push_str("#include <stdint.h>\n#include <stddef.h>\n\n");
    output.push_str(
        "#if defined(_WIN32)\n#define NC_EXPORT __declspec(dllexport)\n#else\n#define NC_EXPORT __attribute__((visibility(\"default\")))\n#endif\n\n",
    );
    output.push_str("typedef struct { uint32_t data1; uint16_t data2; uint16_t data3; uint8_t data4[8]; } nc_guid;\n");
    output.push_str("typedef struct { const uint8_t *data; uint32_t len; } nc_static_utf8;\n");
    output.push_str("typedef struct { uint8_t bytes[32]; } nc_fingerprint;\n");
    output.push_str("struct nc_contract_module;\n");
    output.push_str("typedef const struct nc_contract_module *(*nc_get_contract_module)(void);\n");
    output.push_str("typedef struct { nc_guid module_id; nc_fingerprint abi_fingerprint; nc_get_contract_module get; } nc_contract_dependency;\n");
    output.push_str(
        "typedef struct { nc_guid declaration_id; uint32_t version; uint32_t kind; uint32_t threading; nc_static_utf8 qualified_name; } nc_contract_declaration;\n",
    );
    output.push_str("typedef struct nc_contract_module { uint32_t description_abi_version; nc_guid module_id; nc_fingerprint abi_fingerprint; nc_static_utf8 module_name; uint32_t dependency_count; const nc_contract_dependency *dependencies; uint32_t declaration_count; const nc_contract_declaration *declarations; nc_static_utf8 source; } nc_contract_module;\n\n");

    for dependency in &module.dependencies {
        let dependency_getter = dependency_getter(dependency);
        output.push_str(&format!("extern const nc_contract_module *{dependency_getter}(void);\n"));
    }
    if !module.dependencies.is_empty() {
        output.push('\n');
    }

    byte_array(&mut output, "nc_module_name", module.source.name.to_string().as_bytes());
    byte_array(&mut output, "nc_module_source", source.as_bytes());
    for (index, identity) in module.declarations.iter().enumerate() {
        byte_array(&mut output, &format!("nc_declaration_name_{index}"), identity.qualified_name.as_bytes());
    }

    if !module.dependencies.is_empty() {
        output.push_str("\nstatic const nc_contract_dependency nc_dependencies[] = {\n");
        for dependency in &module.dependencies {
            output.push_str(&format!(
                "    {{ {}, {}, {} }},\n",
                guid_initializer(dependency.module_id),
                fingerprint_initializer(&dependency.abi_fingerprint),
                dependency_getter(dependency)
            ));
        }
        output.push_str("};\n");
    }
    if !module.declarations.is_empty() {
        output.push_str("\nstatic const nc_contract_declaration nc_declarations[] = {\n");
        for (index, (identity, declaration)) in module.declarations.iter().zip(&module.source.declarations).enumerate() {
            output.push_str(&format!(
                "    {{ {}, {}u, {}u, {}u, {{ nc_declaration_name_{index}, {}u }} }},\n",
                guid_initializer(identity.id),
                identity.version,
                declaration_kind(&declaration.kind),
                declaration_threading(&declaration.kind),
                identity.qualified_name.len()
            ));
        }
        output.push_str("};\n");
    }
    output.push_str("\nstatic const nc_contract_module nc_module = {\n");
    output.push_str("    1u,\n");
    output.push_str(&format!("    {},\n", guid_initializer(module.module_id)));
    output.push_str(&format!("    {},\n", fingerprint_initializer(&module.abi_fingerprint)));
    output.push_str(&format!("    {{ nc_module_name, {}u }},\n", module.source.name.to_string().len()));
    if module.dependencies.is_empty() {
        output.push_str("    0u, 0,\n");
    } else {
        output.push_str(&format!("    {}u, nc_dependencies,\n", module.dependencies.len()));
    }
    if module.declarations.is_empty() {
        output.push_str("    0u, 0,\n");
    } else {
        output.push_str(&format!("    {}u, nc_declarations,\n", module.declarations.len()));
    }
    output.push_str(&format!("    {{ nc_module_source, {}u }}\n", source.len()));
    output.push_str("};\n\n");
    output.push_str(&format!("NC_EXPORT const nc_contract_module *{getter}(void) {{\n    return &nc_module;\n}}\n"));
    output
}

/// Returns the exact getter symbol imported for one dependency revision.
fn dependency_getter(dependency: &neocogi_idl::ResolvedDependency) -> String {
    format!(
        "nc_contract_{}_{}_get_v1",
        dependency.module_name.replace('.', "_"),
        fingerprint_hex(&dependency.abi_fingerprint)
    )
}

/// Writes one static byte array without relying on C string escaping.
fn byte_array(output: &mut String, name: &str, bytes: &[u8]) {
    output.push_str(&format!("static const uint8_t {name}[] = {{"));
    if bytes.is_empty() {
        output.push('0');
    } else {
        for (index, byte) in bytes.iter().enumerate() {
            if index != 0 {
                output.push_str(", ");
            }
            output.push_str(&byte.to_string());
        }
    }
    output.push_str("};\n");
}

/// Renders a C aggregate initializer for one ABI GUID.
fn guid_initializer(guid: NcGuid) -> String {
    let (data1, data2, data3, data4) = guid.fields();
    let bytes = data4.iter().map(u8::to_string).collect::<Vec<_>>().join(", ");
    format!("{{ {data1}u, {data2}u, {data3}u, {{ {bytes} }} }}")
}

/// Renders a C aggregate initializer for one fingerprint.
fn fingerprint_initializer(fingerprint: &[u8; 32]) -> String {
    let bytes = fingerprint.iter().map(u8::to_string).collect::<Vec<_>>().join(", ");
    format!("{{ {{ {bytes} }} }}")
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
        DeclarationKind::Interface(_) => 3,
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
        DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_) | DeclarationKind::Interface(_) => u32::MAX,
    }
}

/// Converts one resolved IDL threading capability to its stable raw value.
fn threading_value(threading: Threading) -> u32 {
    match threading {
        Threading::Local => 0,
        Threading::Shared => 1,
    }
}

/// Builds a shared library and any platform linker companion files.
fn compile_dynamic(
    target: &str,
    target_os: &str,
    link_name: &str,
    source: &Path,
    dependencies: &[ArtifactRecord],
    output_dir: &Path,
) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
    let mut build = cc::Build::new();
    build.target(target).cargo_metadata(false);
    let compiler = build.get_compiler();
    let mut command = compiler.to_command();
    let mut linker_files = Vec::new();
    let library = if compiler.is_like_msvc() {
        let library = output_dir.join(format!("{link_name}.dll"));
        let import_library = output_dir.join(format!("{link_name}.lib"));
        command.arg("/nologo").arg("/LD").arg(source).arg("/link");
        command.arg(format!("/OUT:{}", library.display()));
        command.arg(format!("/IMPLIB:{}", import_library.display()));
        for dependency in dependencies {
            command.arg(dependency.linker_file()?);
        }
        linker_files.push(import_library);
        library
    } else if target_os == "windows" {
        let library = output_dir.join(format!("{link_name}.dll"));
        let import_library = output_dir.join(format!("lib{link_name}.dll.a"));
        command.arg("-shared").arg(source).arg("-o").arg(&library);
        command.arg(format!("-Wl,--out-implib,{}", import_library.display()));
        for dependency in dependencies {
            command.arg(dependency.linker_file()?);
        }
        linker_files.push(import_library);
        library
    } else if target_os == "macos" {
        let library = output_dir.join(format!("lib{link_name}.dylib"));
        command.arg("-dynamiclib").arg("-fPIC").arg(source).arg("-o").arg(&library);
        command.arg(format!("-Wl,-install_name,@rpath/lib{link_name}.dylib"));
        for dependency in dependencies {
            command.arg(&dependency.library);
        }
        library
    } else {
        let library = output_dir.join(format!("lib{link_name}.so"));
        command.arg("-shared").arg("-fPIC").arg(source).arg("-o").arg(&library);
        command.arg(format!("-Wl,-soname,lib{link_name}.so"));
        for dependency in dependencies {
            command.arg(&dependency.library);
        }
        library
    };
    run(&mut command, "compile dynamic contract description")?;
    Ok((library, linker_files))
}

/// Builds a static archive for monolith and WebAssembly targets.
fn compile_static(target: &str, link_name: &str, source: &Path, output_dir: &Path) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
    let mut build = cc::Build::new();
    build.target(target).out_dir(output_dir).cargo_metadata(false).file(source).compile(link_name);
    let library = if target.contains("msvc") {
        output_dir.join(format!("{link_name}.lib"))
    } else {
        output_dir.join(format!("lib{link_name}.a"))
    };
    Ok((library.clone(), vec![library]))
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

/// Replaces generated native source only when its bytes changed.
fn write_changed(path: &Path, contents: &[u8]) -> Result<(), BuildError> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|source| BuildError::Io {
        operation: "write generated native source",
        path: path.to_owned(),
        source,
    })
}
