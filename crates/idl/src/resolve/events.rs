//! Event declarations describe emitted immutable interfaces, never receiver specializations.
use super::*;
use std::collections::HashSet;

/// Checks each advertised interface and its complete getter ancestry once.
/// Event edges are semantic dependencies, not interface layout/inheritance edges.
pub(super) fn validate(index: usize, source: &crate::Interface, entries: &[ModuleEntry], owner: &str) -> Result<(), ResolutionError> {
    let mut emitted = HashSet::new();
    for name in &source.events {
        let symbol = resolve_named(index, name, entries)?;
        let invalid = |message: String| {
            source_error(
                &entries[index].module,
                "EIDL065",
                &name.span,
                message,
                "declare an immutable interface derived from neocogi.com.events.IEvent",
            )
        };
        if !emitted.insert(symbol.id) {
            return Err(invalid(format!("{owner} repeats emitted interface {}", name.rust_path())));
        }
        let mut visited = HashSet::new();
        let mut getters = HashSet::new();
        if !inspect(symbol, entries, &mut visited, &mut getters)? {
            return Err(invalid(format!("{} is not derived from IEvent", name.rust_path())));
        }
    }
    Ok(())
}

/// Verifies immutable getter contracts without traversing emitted-event edges recursively.
/// A visited set bounds malformed inheritance cycles until the normal layout validator reports them.
fn inspect(symbol: Symbol, entries: &[ModuleEntry], visited: &mut HashSet<NcGuid>, getters: &mut HashSet<String>) -> Result<bool, ResolutionError> {
    if !visited.insert(symbol.id) {
        return Ok(false);
    }
    let module = &entries[symbol.module].module;
    let declaration = &module.declarations[symbol.declaration];
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Ok(false);
    };
    let mut event = module.name.qualify(&declaration.name) == "neocogi.com.events.IEvent";
    for base in &interface.bases {
        event |= inspect(resolve_named(symbol.module, base, entries)?, entries, visited, getters)?;
    }
    for method in &interface.methods {
        if method.receiver != crate::Receiver::Shared
            || !method.parameters.is_empty()
            || matches!(
                method.output,
                AbiType::ComBox(_) | AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::OutputSlice(_)
            )
        {
            return Err(source_error(
                module,
                "EIDL065",
                &method.span,
                format!(
                    "event getter {}::{} must be immutable, parameterless and return a repeatable value",
                    declaration.name, method.name
                ),
                "event snapshots expose immutable zero-argument getters",
            ));
        }
        if !getters.insert(method.name.clone()) {
            return Err(source_error(
                module,
                "EIDL065",
                &method.span,
                format!("event getter {} is ambiguous across inherited interfaces", method.name),
                "event getter names must be unique",
            ));
        }
    }
    Ok(event)
}
