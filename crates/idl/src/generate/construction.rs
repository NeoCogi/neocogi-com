//! Rust rendering of constructor facts already validated by IDL resolution.
//! Public names come from the same helper entries visited by collision checks.
use super::*;
use crate::resolve::construction::Helper;

/// Adds ergonomic owned conversion to each closed EventBinding specialization.
/// Core's Option is qualified so shadowable IDL names cannot alter helper types.
pub(super) fn value_helpers(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let Some(binding) = module.construction.bindings.get(&declaration.name) else {
        return Ok(quote! {});
    };
    let name = rust_ident(&declaration.name)?;
    let queue = named_rust_type(graph, module, binding.queue.get(graph.graph), true)?;
    Ok(quote! {
        impl #name {
            /// Retains the final constructor receiver, or chooses permanent non-delivery.
            pub fn from_queue(queue: ::core::option::Option<::neocogi_com::ComRef<'_, #queue>>) -> Self {
                match queue { ::core::option::Option::Some(queue) => Self::Observed(queue.add_ref()), ::core::option::Option::None => Self::Unobserved }
            }
            /// Transfers the constructor's owned reference into producer-local storage.
            pub fn into_queue(self) -> ::core::option::Option<::neocogi_com::ComPtr<#queue>> {
                match self { Self::Observed(queue) => ::core::option::Option::Some(queue), Self::Unobserved => ::core::option::Option::None }
            }
        }
    })
}

/// Emits exactly the helpers reserved in the product's resolved constructor plan.
pub(super) fn aliases(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let Some(plan) = module.construction.products.get(&declaration.name) else {
        return Ok(quote! {});
    };
    let binding = plan.binding;
    let value = named_rust_type(graph, module, binding.value.get(graph.graph), false)?;
    let queue = named_rust_type(graph, module, binding.queue.get(graph.graph), true)?;
    let queue_trait = named_rust_type(graph, module, binding.queue.get(graph.graph), false)?;
    let mut provider: syn::Path = syn::parse2(queue_trait.clone()).map_err(|error| GenerateError::new(error.to_string()))?;
    let last = provider.segments.last_mut().expect("named queue path is nonempty");
    last.ident = rust_ident(&format!("{}VtblFor", last.ident))?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let helpers = plan
        .helpers
        .iter()
        .map(|helper| {
            let name = rust_ident(helper.name())?;
            Ok(match helper {
                Helper::Binding(_) => quote! {
                    /// Permanent observed/unobserved choice supplied to this product's constructor.
                    pub type #name = #value;
                },
                Helper::QueueAbi(_) => quote! {
                    /// Exact receiver accepted when this product is constructed.
                    pub type #name = #queue;
                },
                Helper::Queue(_) => quote! {
                    /// Typed receiver implementation contract owned by the application.
                    pub use #queue_trait as #name;
                },
                Helper::QueueProvider(_) => quote! {
                    /// Vtable provider for local implementations using the readable queue alias.
                    pub use #provider as #name;
                },
                Helper::Port(_) => quote! {
                    /// Shared-service producer storage using WeakComPtr; exclusive widgets keep their queue and WeakComBox directly.
                    pub type #name = ::neocogi_com::events::EventPort<#queue>;
                },
            })
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;
    Ok(quote! {
        #(#helpers)*
        impl ::neocogi_com::events::EventSource for #abi {
            /// Constructor-selected receiver; this adds no interface to the producer.
            type Queue = #queue;
        }
    })
}
