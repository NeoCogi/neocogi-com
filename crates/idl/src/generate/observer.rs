//! One typed destination per producer, derived from its declared output messages.

use super::*;

/// Emits the local event sum and an owned COM adapter with its callback inline.
/// Each observer method reuses its payload's checked boundary decoder; no Rust
/// enum, closure, or channel representation is exposed in the public vtable.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, source: &Declaration, callables: &[Callable]) -> Result<TokenStream, GenerateError> {
    let event = rust_ident(&format!("{}Event", source.name))?;
    let observer = rust_ident(&format!("{}Observer", source.name))?;
    let abi = rust_ident(&format!("{}ObserverAbi", source.name))?;
    let adapter = rust_ident(&format!("__{}Observer", source.name))?;
    let mut variants = Vec::new();
    let mut methods = Vec::new();
    let mut posts = Vec::new();
    for callable in callables {
        let Callable::Port(port) = callable else { continue };
        let resolved = resolve(graph, module, &port.message)?;
        let payload = named_rust_type(graph, module, resolved, false)?;
        let method = rust_ident(&port.name)?;
        let variant_name = port
            .name
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<String>();
        let variant = rust_ident(&variant_name)?;
        let documentation = docs(&port.docs);
        variants.push(quote! { #documentation #variant(#payload) });
        methods.push(quote! {
            #documentation
            fn #method(&mut self, event: ::neocogi_com::NcMessage<'_, #payload>) -> Result<(), ::neocogi_com::NcStatus> {
                // Copy borrowed fields before passing the snapshot to local Rust.
                (self.consume)(#event::#variant(event.to_owned()?))
            }
        });
        posts.push(quote! { Self::#variant(value) => destination.#method(::neocogi_com::NcMessage::new(value)) });
    }
    Ok(quote! {
        /// Owned local snapshots of this producer's outputs, in declared order.
        /// This enum is a Rust convenience and never crosses the COM ABI.
        #[derive(Clone)]
        pub enum #event { #(#variants),* }

        impl #event {
            /// Owns one callback in the observer allocation for all output kinds.
            /// The callback must own its captures; it cannot borrow a stack frame.
            pub fn observer<F>(consume: F) -> Result<::neocogi_com::ComPtr<#abi>, ::neocogi_com::NcStatus>
            where F: FnMut(Self) -> Result<(), ::neocogi_com::NcStatus> + 'static {
                #adapter::from_value(#adapter { consume }).query()
            }

            /// Posts through the observer retained at construction, or does nothing
            /// when the widget was explicitly created without a consumer.
            pub fn post_to(&self, destination: &mut Option<::neocogi_com::ComPtr<#abi>>) -> Result<(), ::neocogi_com::NcStatus> {
                match destination {
                    Some(destination) => match self { #(#posts),* },
                    None => Ok(()),
                }
            }
        }

        #[::neocogi_com::com_object(#observer)]
        /// One COM-owned callable; no separate boxed callback or local downcast.
        struct #adapter<F>
        where F: FnMut(#event) -> Result<(), ::neocogi_com::NcStatus> + 'static {
            /// Moves each decoded snapshot into the owner's chosen transport.
            consume: F,
        }
        impl<F> #observer for #adapter<F>
        where F: FnMut(#event) -> Result<(), ::neocogi_com::NcStatus> + 'static {
            #(#methods)*
        }
    })
}
