//! The sole lexical lookup boundary, shared by generic and ordinary declarations.
use super::*;
use crate::binding::{Binding, DeclarationKey};

/// Binds every authored type while collecting independent source-aware failures.
/// Symbol tables are complete before this pass. Moving a declaration out while
/// walking it therefore leaves all lexical owners available without a second AST.
pub(super) fn bind(entries: &mut [ModuleEntry]) -> Result<Vec<Diagnostic>, ResolutionError> {
    let mut diagnostics = Vec::new();
    for index in 0..entries.len() {
        let mut declarations = std::mem::take(&mut entries[index].module.declarations);
        for declaration in &mut declarations {
            let parameters = declaration.parameters.clone();
            crate::specialize::visit_types(declaration, &mut |ty| {
                bind_type(index, &parameters, ty, entries, &mut diagnostics);
                Ok(())
            })?;
        }
        entries[index].module.declarations = declarations;
    }
    Ok(diagnostics)
}

/// Assigns semantic ownership once; recursive arguments retain their own scopes.
fn bind_type(index: usize, parameters: &[String], ty: &mut AbiType, entries: &[ModuleEntry], diagnostics: &mut Vec<Diagnostic>) {
    match ty {
        AbiType::Named(name) => {
            let result = bind_name(index, parameters, name, entries);
            match result {
                Ok(binding) => name.binding = binding,
                Err(error) => diagnostics.extend(error.diagnostics),
            }
            for argument in &mut name.arguments {
                bind_type(index, parameters, argument, entries, diagnostics);
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
        | AbiType::Array { element: value, .. } => {
            bind_type(index, parameters, value, entries, diagnostics);
        }
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
}

/// Parameters shadow bare declarations/intrinsics; qualified names always name owners.
fn bind_name(index: usize, parameters: &[String], name: &NamedType, entries: &[ModuleEntry]) -> Result<Binding, ResolutionError> {
    let entry = &entries[index];
    let parameter = match name.segments.as_slice() {
        [local] => parameters.iter().position(|parameter| parameter == local),
        _ => None,
    };
    if let Some(position) = parameter {
        require_arity(index, name, 0, entries, "a type parameter cannot accept arguments")?;
        return Ok(Binding::Parameter {
            position: u32_len(position, &entry.name, "parameter position")?,
        });
    }
    if let Some(intrinsic) = crate::binding::intrinsic(&name.segments) {
        require_arity(index, name, 0, entries, "built-in types do not accept type arguments")?;
        return Ok(Binding::Intrinsic(intrinsic));
    }
    let symbol = lookup_source(index, name, entries)?;
    require_arity(
        index,
        name,
        symbol.parameters,
        entries,
        &format!(
            "{} expects {} type arguments, received {}",
            name.rust_path(),
            symbol.parameters,
            name.arguments.len()
        ),
    )?;
    let key = DeclarationKey {
        module: symbol.module,
        name: name.segments.last().expect("parsed nonempty type path").clone(),
    };
    Ok(match symbol.parameters {
        0 => Binding::Declaration(key),
        _ => Binding::Application { template: key, projection: None },
    })
}

/// Arity is lexical well-formedness even when no consumer instantiates a template.
fn require_arity(index: usize, name: &NamedType, expected: usize, entries: &[ModuleEntry], message: &str) -> Result<(), ResolutionError> {
    match name.arguments.len() == expected {
        true => Ok(()),
        false => Err(source_error(&entries[index].module, "EIDL060", &name.span, message, "invalid type arguments")),
    }
}

/// Looks up authored spelling exactly once, before substitution changes any owners.
fn lookup_source(index: usize, name: &NamedType, entries: &[ModuleEntry]) -> Result<Symbol, ResolutionError> {
    let entry = &entries[index];
    if let [local] = name.segments.as_slice() {
        return entry
            .local_symbols
            .get(local)
            .or_else(|| entry.imported_symbols.get(local))
            .copied()
            .ok_or_else(|| {
                source_error(
                    &entry.module,
                    "EIDL070",
                    &name.span,
                    format!("unknown IDL type {local:?}"),
                    "type is not declared or imported",
                )
            });
    }
    let owner = name.segments[..name.segments.len() - 1].join(".");
    let local = name.segments.last().expect("parsed nonempty type path");
    let module = entries.iter().position(|entry| entry.name == owner).ok_or_else(|| {
        source_error(
            &entry.module,
            "EIDL071",
            &name.span,
            format!("unknown IDL module {owner:?}"),
            "module is not present in the contract graph",
        )
    })?;
    if module != index && !entry.dependencies.contains(&module) {
        return Err(source_error(
            &entry.module,
            "EIDL072",
            &name.span,
            format!("type {} is used without declaring its external module", name.rust_path()),
            "missing external dependency",
        ));
    }
    entries[module].local_symbols.get(local).copied().ok_or_else(|| {
        source_error(
            &entry.module,
            "EIDL070",
            &name.span,
            format!("unknown IDL type {}", name.rust_path()),
            "module does not declare this type",
        )
    })
}
