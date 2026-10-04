//! Resolve constructor/library associations once, before validating public names.
//!
//! These are ordinary EventBinding/IEventReceiver contracts, not parser syntax.
//! The product's declaring module owns the association; imported factories cannot
//! change another module's helpers or metadata. Stored symbols address the frozen
//! resolved graph, so generation does not rediscover relationships from methods.
use super::*;
use std::collections::BTreeMap;

/// Canonical ordinary library enum whose closed shape selects event delivery.
const BINDING: &str = "neocogi.com.events.EventBinding";

/// Validated closed binding and the declarations needed by every consumer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Binding {
    /// Closed EventBinding specialization used by generated construction helpers.
    pub value: Symbol,
    /// Common IEventReceiver interface owned by the binding's Observed case.
    pub queue: Symbol,
    /// Public producer projection borrowed during posting.
    pub origin: Symbol,
    /// Complete declared event value acquired by the queue.
    pub event: Symbol,
}

/// One public helper to emit and reserve, including its already-decided name.
/// Generation exhaustively matches these cases; validation visits every name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Helper {
    /// Readable alias for the closed construction policy enum.
    Binding(String),
    /// Shared-service helper; emitted only when an associated constructor returns ComPtr.
    Port(String),
}
impl Helper {
    /// Borrows the exact identifier that both validation and emission consume.
    pub fn name(&self) -> &str {
        match self {
            Self::Binding(name) | Self::Port(name) => name,
        }
    }
}

/// One product's constructor association and complete emitted helper inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Constructor {
    /// Validated binding selected by every associated local constructor.
    pub binding: Binding,
    /// First associated method, retained for conflicting-constructor diagnostics.
    pub source: SourceSpan,
    /// Exactly the public aliases the generator will emit for this product.
    pub helpers: Vec<Helper>,
}

/// Compile-time construction facts owned by one resolved module.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Construction {
    /// Closed binding definitions, including bindings not used by a local factory.
    pub bindings: BTreeMap<String, Binding>,
    /// Local product names mapped to their one consistent constructor association.
    pub products: BTreeMap<String, Constructor>,
}

/// Resolves library shapes first, then visits each possible constructor once.
/// Module/declaration ordering is already canonical and remains fixed afterward.
pub(super) fn resolve(graph: &ResolvedModules) -> Result<Vec<Construction>, ResolutionError> {
    let mut plans = vec![Construction::default(); graph.modules.len()];
    for (index, module) in graph.modules.iter().enumerate() {
        for declaration in &module.source.declarations {
            if let Some(binding) = binding(graph, module, declaration)? {
                plans[index].bindings.insert(declaration.name.clone(), binding);
            }
        }
    }
    for (index, module) in graph.modules.iter().enumerate() {
        for declaration in &module.source.declarations {
            let DeclarationKind::Interface(interface) = &declaration.kind else { continue };
            for method in &interface.methods {
                let (AbiType::ComPtr(output) | AbiType::WeakComPtr(output) | AbiType::ComBox(output) | AbiType::WeakComBox(output)) = &method.output else {
                    continue;
                };
                let AbiType::Named(output) = output.as_ref() else { continue };
                let Some(product) = graph.resolve_symbol(module, output) else { continue };
                if product.module != index {
                    // A downstream factory cannot modify its dependency's public
                    // product names, event type, or published metadata.
                    continue;
                }
                let product_name = &product.get(graph).declaration.name;
                let mut selected = None;
                for parameter in &method.parameters {
                    let AbiType::Borrowed(value) = &parameter.ty else { continue };
                    let AbiType::Named(value) = value.as_ref() else { continue };
                    let Some(value) = graph.resolve_symbol(module, value) else { continue };
                    let Some(candidate) = plans[value.module].bindings.get(&value.get(graph).declaration.name).copied() else {
                        continue;
                    };
                    if let Some(previous) = selected {
                        return Err(failure(
                            graph,
                            Diagnostic::new(
                                "EIDL065",
                                format!("constructor {}::{} accepts multiple event bindings", declaration.name, method.name,),
                            )
                            .primary(&parameter.span, "second event binding")
                            .secondary(previous, "first event binding"),
                        ));
                    }
                    selected = Some(&parameter.span);
                    if candidate.origin.id != product.id {
                        return Err(failure(
                            graph,
                            Diagnostic::new(
                                "EIDL065",
                                format!(
                                    "constructor {}::{} must bind its returned product as event origin",
                                    declaration.name, method.name,
                                ),
                            )
                            .primary(&parameter.span, "binding names a different origin")
                            .secondary(&method.span, "constructor returns this product"),
                        ));
                    }
                    let plan = plans[index].products.entry(product_name.clone()).or_insert_with(|| Constructor {
                        binding: candidate,
                        source: method.span.clone(),
                        helpers: vec![
                            Helper::Binding(format!("{product_name}EventBinding")),
                        ],
                    });
                    if plan.binding.event.id != candidate.event.id {
                        return Err(failure(
                            graph,
                            Diagnostic::new("EIDL065", format!("constructors of {product_name} disagree on their event type"))
                                .primary(&method.span, "conflicting constructor")
                                .secondary(&plan.source, "previous constructor selects a different event"),
                        ));
                    }
                    // EventPort stores WeakComPtr, so it is useful only for a
                    // shared-owner construction path. Exclusive products retain
                    // their queue and WeakComBox directly. A later shared factory
                    // can add this helper once, independent of declaration order.
                    if matches!(method.output, AbiType::ComPtr(_)) && !plan.helpers.iter().any(|helper| matches!(helper, Helper::Port(_))) {
                        plan.helpers.push(Helper::Port(format!("{product_name}EventPort")));
                    }
                }
            }
        }
    }
    Ok(plans)
}

/// Checks the canonical library enum and resolves its exact concrete symbols.
/// Ordinary user enums with similar spellings do not trigger this adapter.
fn binding(graph: &ResolvedModules, module: &ResolvedModule, declaration: &Declaration) -> Result<Option<Binding>, ResolutionError> {
    let Some(specialization) = &declaration.specialization else { return Ok(None) };
    if specialization.template.segments.join(".") != BINDING {
        return Ok(None);
    }
    let invalid = |message| {
        failure(
            graph,
            Diagnostic::new("EIDL065", message).primary(&declaration.span, "invalid closed EventBinding contract"),
        )
    };
    let [AbiType::Named(origin), AbiType::Named(event)] = specialization.arguments.as_slice() else {
        return Err(invalid("EventBinding requires an interface origin and one declared event value"));
    };
    let origin = graph
        .resolve_symbol(module, origin)
        .ok_or_else(|| invalid("EventBinding origin must name a declared interface"))?;
    let event = graph
        .resolve_symbol(module, event)
        .ok_or_else(|| invalid("EventBinding event must name a declared value"))?;
    if origin.kind != SymbolKind::Interface || event.kind != SymbolKind::Value {
        return Err(invalid("EventBinding requires an interface origin and one declared event value"));
    }
    let DeclarationKind::Enum(enumeration) = &declaration.kind else {
        return Err(invalid("EventBinding must be an enum"));
    };
    let [absent, observed] = enumeration.variants.as_slice() else {
        return Err(invalid("EventBinding must contain Unobserved and Observed"));
    };
    if absent.name != "Unobserved" || absent.fields != VariantFields::Unit || observed.name != "Observed" {
        return Err(invalid("EventBinding variants disagree with their construction contract"));
    }
    let VariantFields::Tuple(fields) = &observed.fields else {
        return Err(invalid("EventBinding.Observed must own a typed queue"));
    };
    let [AbiType::ComPtr(queue)] = fields.as_slice() else {
        return Err(invalid("EventBinding.Observed must own one ComPtr"));
    };
    let AbiType::Named(queue) = queue.as_ref() else {
        return Err(invalid("EventBinding queue must name an interface"));
    };
    let queue = graph
        .resolve_symbol(module, queue)
        .ok_or_else(|| invalid("EventBinding queue must name a declared interface"))?;
    let expected = graph.resolve_symbol(module, &NamedType {
        segments: vec!["neocogi".into(), "com".into(), "events".into(), "IEventReceiver".into()],
        arguments: Vec::new(), span: declaration.span.clone(),
    });
    if queue.kind != SymbolKind::Interface || expected.map(|symbol| symbol.id) != Some(queue.id) {
        return Err(invalid("EventBinding.Observed must own the common IEventReceiver"));
    }
    let value = graph
        .resolve_symbol(
            module,
            &NamedType {
                segments: vec![declaration.name.clone()],
                arguments: Vec::new(),
                span: declaration.span.clone(),
            },
        )
        .expect("binding declaration belongs to this resolved module");
    Ok(Some(Binding { value, queue, origin, event }))
}

/// Includes template and use-site sources so cross-module labels remain readable.
fn failure(graph: &ResolvedModules, diagnostic: Diagnostic) -> ResolutionError {
    let diagnostic = graph
        .modules
        .iter()
        .fold(diagnostic, |diagnostic, module| diagnostic.with_sources(&module.source.sources));
    ResolutionError { diagnostics: vec![diagnostic] }
}
