//! Construction-bound typed event receivers, derived from ordinary method types.
//!
//! EventBinding<O, E> is a library enum. The backend recognizes its identity to
//! publish receiver helpers and metadata; neither the parser nor resolver gains
//! UI syntax. Constructor association is local to the product's contract module,
//! so importing another factory cannot change already-published type metadata.

use super::*;

/// Canonical library identity of the permanent observed/unobserved choice.
const BINDING: &str = "neocogi.com.events.EventBinding";

/// One constructor argument and the exact queue/value that its enum owns.
#[derive(Clone, Copy)]
pub(super) struct Binding<'a> {
    /// Closed generic value passed to construction through checked value ABI.
    pub value: ResolvedDeclaration<'a>,
    /// Concrete IEventQueue<O, E> specialization retained by the producer.
    pub queue: ResolvedDeclaration<'a>,
    /// Event root whose reachable value graph is published for inspection.
    pub origin: ResolvedDeclaration<'a>,
    /// Complete event value acquired by the receiver.
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
    let [AbiType::Named(origin), AbiType::Named(event)] = specialization.arguments.as_slice() else {
        return Err(GenerateError::new("EventBinding requires an interface origin and one declared event value"));
    };
    let origin = resolve(graph, value.module, origin)?;
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
        return Err(GenerateError::new("EventBinding.Observed must own a typed queue"));
    };
    let [AbiType::ComPtr(queue)] = fields.as_slice() else {
        return Err(GenerateError::new("EventBinding.Observed must own one ComPtr"));
    };
    let AbiType::Named(queue) = queue.as_ref() else {
        return Err(GenerateError::new("EventBinding queue must name an interface"));
    };
    let queue = resolve(graph, value.module, queue)?;
    let Some(specialization) = &queue.declaration.specialization else {
        return Err(GenerateError::new("EventBinding queue must specialize IEventQueue"));
    };
    if specialization.template.segments.join(".") != "neocogi.com.events.IEventQueue"
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
        return Err(GenerateError::new("EventBinding queue must receive its declared event type"));
    }
    Ok(Some(Binding { value, queue, origin, event }))
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
            let (AbiType::ComPtr(output) | AbiType::ComBox(output) | AbiType::WeakComBox(output)) = &method.output else { continue };
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
                if candidate.origin.identity.id != product_id {
                    return Err(GenerateError::new(format!(
                        "constructor {}::{} must bind its returned product as event origin",
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
    let queue = named_rust_type(graph, module, binding.queue, true)?;
    Ok(quote! {
        impl #name {
            /// Retains the final constructor receiver, or chooses permanent non-delivery.
            pub fn from_queue(queue: Option<::neocogi_com::ComRef<'_, #queue>>) -> Self {
                match queue { Some(queue) => Self::Observed(queue.add_ref()), None => Self::Unobserved }
            }
            /// Transfers the constructor's owned reference into producer-local storage.
            pub fn into_queue(self) -> Option<::neocogi_com::ComPtr<#queue>> {
                match self { Self::Observed(queue) => Some(queue), Self::Unobserved => None }
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
    let queue = named_rust_type(graph, module, binding.queue, true)?;
    let queue_trait = named_rust_type(graph, module, binding.queue, false)?;
    let binding_alias = rust_ident(&format!("{}EventBinding", declaration.name))?;
    let queue_alias = rust_ident(&format!("{}EventQueueAbi", declaration.name))?;
    let trait_alias = rust_ident(&format!("{}EventQueue", declaration.name))?;
    let provider_alias = rust_ident(&format!("{}EventQueueVtblFor", declaration.name))?;
    let mut provider: syn::Path = syn::parse2(queue_trait.clone()).map_err(|error| GenerateError::new(error.to_string()))?;
    let last = provider.segments.last_mut().expect("named queue path is nonempty");
    last.ident = rust_ident(&format!("{}VtblFor", last.ident))?;
    let port = rust_ident(&format!("{}EventPort", declaration.name))?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let receiver = quote! {
        /// Exact receiver accepted when this product is constructed.
        pub type #queue_alias = #queue;
        /// Typed receiver implementation contract owned by the application.
        pub use #queue_trait as #trait_alias;
        /// Vtable provider for local implementations using the readable queue alias.
        pub use #provider as #provider_alias;
        /// Private producer storage initialized with its constructor's final queue.
        pub type #port = ::neocogi_com::events::EventPort<#queue>;
        impl ::neocogi_com::events::EventSource for #abi {
            /// Constructor-selected receiver; this adds no interface to the producer.
            type Queue = #queue;
        }
    };
    Ok(quote! {
        /// Permanent observed/unobserved choice supplied to this product's constructor.
        pub type #binding_alias = #value;
        #receiver
    })
}
