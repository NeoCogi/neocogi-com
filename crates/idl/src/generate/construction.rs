//! Construction-bound typed event receivers, derived from ordinary method types.
//!
//! EventBinding<E> is a library enum. The backend recognizes its identity to
//! publish receiver helpers and metadata; neither the parser nor resolver gains
//! UI syntax. Constructor association is local to the product's contract module,
//! so importing another factory cannot change already-published type metadata.

use super::*;

/// Canonical library identity of the permanent observed/unobserved choice.
const BINDING: &str = "neocogi.com.events.EventBinding";

/// One constructor argument and the exact sink/value that its enum owns.
#[derive(Clone, Copy)]
pub(super) struct Binding<'a> {
    /// Closed generic value passed to construction through checked value ABI.
    pub value: ResolvedDeclaration<'a>,
    /// Concrete IMessageSink<E> specialization retained by the producer.
    pub sink: ResolvedDeclaration<'a>,
    /// Event root whose reachable value graph is published for inspection.
    pub event: ResolvedDeclaration<'a>,
}

/// Recognizes the library enum by identity and verifies its normative shape.
fn binding<'a>(graph: &'a Generation<'_>, value: ResolvedDeclaration<'a>) -> Result<Option<Binding<'a>>, GenerateError> {
    let Some(specialization) = &value.declaration.specialization else {
        return Ok(None);
    };
    if specialization.template.segments.join(".") != BINDING {
        return Ok(None);
    }
    let [AbiType::Named(event)] = specialization.arguments.as_slice() else {
        return Err(GenerateError::new("EventBinding requires one declared event value"));
    };
    let event = resolve(graph, value.module, event)?;
    let DeclarationKind::Enum(enumeration) = &value.declaration.kind else {
        return Err(GenerateError::new("EventBinding must be an enum"));
    };
    let [absent, observed] = enumeration.variants.as_slice() else {
        return Err(GenerateError::new("EventBinding must contain Unobserved and Observed"));
    };
    if absent.name != "Unobserved" || absent.fields != VariantFields::Unit || observed.name != "Observed" {
        return Err(GenerateError::new("EventBinding variants disagree with their construction contract"));
    }
    let VariantFields::Tuple(fields) = &observed.fields else {
        return Err(GenerateError::new("EventBinding.Observed must own a typed sink"));
    };
    let [AbiType::ComPtr(sink)] = fields.as_slice() else {
        return Err(GenerateError::new("EventBinding.Observed must own one ComPtr"));
    };
    let AbiType::Named(sink) = sink.as_ref() else {
        return Err(GenerateError::new("EventBinding sink must name an interface"));
    };
    let sink = resolve(graph, value.module, sink)?;
    let Some(specialization) = &sink.declaration.specialization else {
        return Err(GenerateError::new("EventBinding sink must specialize IMessageSink"));
    };
    if specialization.template.segments.join(".") != "neocogi.com.events.IMessageSink"
        || specialization.arguments.iter().map(AbiType::source_name).collect::<Vec<_>>()
            != value
                .declaration
                .specialization
                .as_ref()
                .unwrap()
                .arguments
                .iter()
                .map(AbiType::source_name)
                .collect::<Vec<_>>()
    {
        return Err(GenerateError::new("EventBinding sink must receive its declared event type"));
    }
    Ok(Some(Binding { value, sink, event }))
}

/// Finds the sole event type accepted by constructors returning this projection.
/// All methods returning the exact product identity participate, independently
/// of spelling or whether their owner is an interface or a factory. Different
/// methods may expose different initialization data, but never different events.
pub(super) fn find<'a>(graph: &'a Generation<'_>, module: &'a ResolvedModule, product: &'a Declaration) -> Result<Option<Binding<'a>>, GenerateError> {
    let mut found: Option<Binding<'a>> = None;
    let product_id = module.declaration(&product.name).expect("resolved product identity").id;
    for declaration in &module.source.declarations {
        let DeclarationKind::Interface(interface) = &declaration.kind else { continue };
        for callable in &interface.callables {
            let Callable::Method(method) = callable;
            let AbiType::ComPtr(output) = &method.output else { continue };
            let AbiType::Named(output) = output.as_ref() else { continue };
            if builtin_object_type(output).is_some() || resolve(graph, module, output)?.identity.id != product_id {
                continue;
            }
            let mut method_binding = None;
            for parameter in &method.parameters {
                let AbiType::Borrowed(value) = &parameter.ty else { continue };
                let AbiType::Named(value) = value.as_ref() else { continue };
                if builtin_object_type(value).is_some() || value.segments.as_slice() == ["str"] {
                    continue;
                }
                let Some(candidate) = binding(graph, resolve(graph, module, value)?)? else {
                    continue;
                };
                if method_binding.is_some() {
                    return Err(GenerateError::new(format!(
                        "constructor {}::{} accepts multiple event bindings",
                        declaration.name, method.name
                    )));
                }
                method_binding = Some(candidate);
            }
            if let Some(candidate) = method_binding {
                if found.is_some_and(|previous| previous.event.identity.id != candidate.event.identity.id) {
                    return Err(GenerateError::new(format!("constructors of {} disagree on their event type", product.name)));
                }
                found = Some(candidate);
            }
        }
    }
    Ok(found)
}

/// Adds ergonomic owned conversion to each closed EventBinding specialization.
pub(super) fn value_helpers(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let identity = module.declaration(&declaration.name).expect("resolved declaration");
    let Some(binding) = binding(graph, ResolvedDeclaration { module, declaration, identity })? else {
        return Ok(quote! {});
    };
    let name = rust_ident(&declaration.name)?;
    let sink = named_rust_type(graph, module, binding.sink, true)?;
    Ok(quote! {
        impl #name {
            /// Retains the final constructor receiver, or chooses permanent non-delivery.
            pub fn from_sink(sink: Option<::neocogi_com::ComRef<'_, #sink>>) -> Self {
                match sink { Some(sink) => Self::Observed(sink.add_ref()), None => Self::Unobserved }
            }
            /// Transfers the constructor's owned reference into producer-local storage.
            pub fn into_sink(self) -> Option<::neocogi_com::ComPtr<#sink>> {
                match self { Self::Observed(sink) => Some(sink), Self::Unobserved => None }
            }
        }
    })
}

/// Exposes readable aliases derived from the product's constructor signature.
pub(super) fn aliases(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let Some(binding) = find(graph, module, declaration)? else {
        return Ok(quote! {});
    };
    let value = named_rust_type(graph, module, binding.value, false)?;
    let sink = named_rust_type(graph, module, binding.sink, true)?;
    let sink_trait = named_rust_type(graph, module, binding.sink, false)?;
    let binding_alias = rust_ident(&format!("{}EventBinding", declaration.name))?;
    let sink_alias = rust_ident(&format!("{}SinkAbi", declaration.name))?;
    let trait_alias = rust_ident(&format!("{}Sink", declaration.name))?;
    let provider_alias = rust_ident(&format!("{}SinkVtblFor", declaration.name))?;
    let mut provider: syn::Path = syn::parse2(sink_trait.clone()).map_err(|error| GenerateError::new(error.to_string()))?;
    let last = provider.segments.last_mut().expect("named sink path is nonempty");
    last.ident = rust_ident(&format!("{}VtblFor", last.ident))?;
    let port = rust_ident(&format!("{}EventPort", declaration.name))?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let receiver = quote! {
        /// Exact receiver accepted when this product is constructed.
        pub type #sink_alias = #sink;
        /// Typed receiver implementation contract owned by the application.
        pub use #sink_trait as #trait_alias;
        /// Vtable provider for local implementations using the readable sink alias.
        pub use #provider as #provider_alias;
        /// Private producer storage initialized with its constructor's final sink.
        pub type #port = ::neocogi_com::events::EventPort<#sink>;
        impl ::neocogi_com::events::EventSource for #abi {
            /// Constructor-selected receiver; this adds no interface to the producer.
            type Sink = #sink;
        }
    };
    Ok(quote! {
        /// Permanent observed/unobserved choice supplied to this product's constructor.
        pub type #binding_alias = #value;
        #receiver
    })
}
