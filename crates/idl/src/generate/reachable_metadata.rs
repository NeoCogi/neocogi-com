//! Bound reachable-definition discovery, separate from bounded Rust syntax emission.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// One borrowed DFS request. Names remain unresolved until their request is
/// popped, preserving supplying scope and first-encounter error/projection order.
/// This frontier can scale with sibling width; it owns no copied type tree.
enum Work<'a> {
    /// Visit one concrete supplying interface projection, including its arguments.
    Interface(ResolvedDeclaration<'a>),
    /// Resolve one declared base only after earlier local roots/bases complete.
    Base {
        /// Actual module supplying the base spelling/binding.
        module: &'a ResolvedModule,
        /// Borrowed bound name; scheduling must not reinterpret or eagerly resolve it.
        name: &'a NamedType,
    },
    /// Inspect a field or call root, peeling unary wrappers without native recursion.
    Type {
        /// Actual module supplying the retained type expression.
        module: &'a ResolvedModule,
        /// Borrowed expression; named data expands only after canonical admission.
        ty: &'a IdlType,
    },
}

/// Finds each reachable data definition in canonical-name order for metadata.
/// Interface visits use concrete module-qualified declaration names, while data
/// visits use canonical identities after requesting-module projection preference.
/// Reverse scheduling retains left-to-right DFS, but resolution happens on pop.
/// Syntax emission consumes these same borrowed declarations on its existing
/// worker; no expanded semantic copy or runtime inspection graph is retained.
pub(super) fn definitions<'a>(graph: &Generation<'a>, requested: ResolvedDeclaration<'a>) -> Result<Vec<ResolvedDeclaration<'a>>, GenerateError> {
    let preferred = requested.module;
    let mut visited = BTreeSet::new();
    let mut names = BTreeMap::new();
    let mut work = vec![Work::Interface(requested)];
    while let Some(next) = work.pop() {
        match next {
            Work::Base { module, name } => work.push(Work::Interface(resolve(graph, module, name)?)),
            Work::Interface(value) => {
                if !visited.insert(value.module.source.name.qualify(&value.declaration.name)) {
                    continue;
                }
                let DeclarationKind::Interface(interface) = &value.declaration.kind else {
                    continue;
                };
                // Pop order is arguments, local methods (parameters then output),
                // and finally bases. Scheduling itself performs no resolution.
                for name in interface.bases.iter().rev() {
                    work.push(Work::Base { module: value.module, name });
                }
                for method in interface.methods.iter().rev() {
                    work.push(Work::Type { module: value.module, ty: &method.output });
                    for parameter in method.parameters.iter().rev() {
                        work.push(Work::Type { module: value.module, ty: &parameter.ty });
                    }
                }
                if let Some(specialization) = &value.declaration.specialization {
                    for ty in specialization.arguments.iter().rev() {
                        work.push(Work::Type { module: value.module, ty });
                    }
                }
            }
            Work::Type { module, mut ty } => loop {
                match ty {
                    IdlType::Named(name) if builtin_object_type(&graph.support, name).is_none() && !name.is_str() => {
                        let value = resolve(graph, module, name)?;
                        if !matches!(value.declaration.kind, DeclarationKind::Interface(_)) {
                            // A canonical ValueGraph shape cannot substitute for
                            // this actual supplying Rust projection and its fields.
                            let value = graph.graph.local_projection(preferred, &value.identity.qualified_name).unwrap_or(value);
                            if !names.contains_key(&value.identity.qualified_name) {
                                names.insert(value.identity.qualified_name.clone(), value);
                                for ty in crate::resolve::value_fields(value.declaration).into_iter().rev() {
                                    work.push(Work::Type { module: value.module, ty });
                                }
                            }
                        }
                        break;
                    }
                    IdlType::ComPtr(value)
                    | IdlType::WeakComPtr(value)
                    | IdlType::ComBox(value)
                    | IdlType::Borrowed(value)
                    | IdlType::BorrowedMut(value)
                    | IdlType::Vector(value)
                    | IdlType::UnsizedSlice(value)
                    | IdlType::Slice(value)
                    | IdlType::OutputSlice(value)
                    | IdlType::Array { element: value, .. } => ty = value,
                    _ => break,
                }
            },
        }
    }
    Ok(names.into_values().collect())
}
