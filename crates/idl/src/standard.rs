//! Bundled IDL library resolution, separate from syntax and intrinsic lowering.
//!
//! Prelude names are ordinary declarations after lexical lookup. The bundled
//! source participates in specialization and fingerprints just like a dependency;
//! no Option-specific rule exists in value conversion or metadata generation.
use crate::{AbiType, ExternalModule, Import, Module, ModuleGraphError, ResolutionError, SourceSpan};

#[cfg(test)]
mod tests;

/// Compiler-owned module containing portable generic value declarations.
const LANGUAGE_MODULE: &str = "neocogi.lang";
/// The shipped source is the authoritative definition, including decoder policy.
const LANGUAGE_SOURCE: &str = include_str!("../stdlib/neocogi/lang.ncidl");

/// Whether a module is supplied by this compiler rather than a Cargo contract package.
/// Build integrations must not request a DEP_* graph for these source dependencies.
pub fn is_bundled_module(name: &str) -> bool {
    name == LANGUAGE_MODULE
}

/// Adds the standard source only when used and supplies unshadowed prelude imports.
/// Authored declarations/imports and template parameters take lexical precedence.
/// The compiler owns the reserved module path so another package cannot silently
/// replace the meaning or fingerprint of the shipped standard library.
pub(crate) fn attach(mut modules: Vec<Module>) -> Result<Vec<Module>, ModuleGraphError> {
    // Tested syntax is not proof that the OS can start another syntax worker.
    let standard = crate::parse_module(std::path::Path::new("neocogi/lang.ncidl"), LANGUAGE_SOURCE).map_err(ModuleGraphError::BundledSource)?;
    if let Some(module) = modules.iter().find(|module| is_bundled_module(&module.name.to_string())) {
        return Err(ResolutionError {
            diagnostics: vec![crate::Diagnostic::new("EIDL067", "neocogi.lang is reserved for the bundled IDL library").with_sources(&module.sources)],
        }
        .into());
    }
    let mut needed = false;
    for module in &mut modules {
        let mut dependency = module
            .externals
            .iter()
            .find(|external| is_bundled_module(&external.module))
            .map(|external| external.span.clone())
            .or_else(|| {
                module
                    .imports
                    .iter()
                    .find(|import| is_bundled_module(&import.module))
                    .map(|import| import.span.clone())
            });
        for library in &standard.declarations {
            let shadowed = module.declaration(&library.name).is_some() || module.imports.iter().any(|import| import.name == library.name);
            let mut prelude = None;
            for declaration in &module.declarations {
                let shadowed = shadowed || declaration.parameters.contains(&library.name);
                let mut declaration = declaration.clone();
                crate::specialize::visit_types(&mut declaration, &mut |ty| {
                    references(ty, &library.name, shadowed, &mut prelude, &mut dependency);
                    Ok(())
                })?;
            }
            if let Some(span) = prelude {
                dependency.get_or_insert_with(|| span.clone());
                module.imports.push(Import {
                    span,
                    module: LANGUAGE_MODULE.into(),
                    name: library.name.clone(),
                });
            }
        }
        if let Some(span) = dependency {
            needed = true;
            if !module.externals.iter().any(|external| is_bundled_module(&external.module)) {
                module.externals.push(ExternalModule { span, module: LANGUAGE_MODULE.into() });
            }
        }
    }
    if needed {
        modules.push(standard);
    }
    Ok(modules)
}

/// Searches recursive type arguments without treating a local name as a keyword.
fn references(ty: &AbiType, name: &str, shadowed: bool, prelude: &mut Option<SourceSpan>, dependency: &mut Option<SourceSpan>) {
    match ty {
        AbiType::Named(value) => {
            if value.segments.as_slice() == [name] && !shadowed {
                prelude.get_or_insert_with(|| value.span.clone());
            } else if value.segments.len() == 3 && value.segments[..2] == ["neocogi", "lang"] {
                dependency.get_or_insert_with(|| value.span.clone());
            }
            for argument in &value.arguments {
                references(argument, name, shadowed, prelude, dependency);
            }
        }
        AbiType::ComPtr(value)
        | AbiType::WeakComPtr(value)
        | AbiType::ComBox(value)
        | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => references(value, name, shadowed, prelude, dependency),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
}
