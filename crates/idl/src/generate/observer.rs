//! One construction-time observer receives the typed event stream; public rebinding replaces its sole destination.

use super::*;

/// Emits the local event sum and an owned COM adapter with its callback inline.
/// Each observer method reuses its payload's checked boundary decoder; no Rust
/// enum, closure, or channel representation is exposed in the public vtable.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, source: &Declaration, callables: &[Callable]) -> Result<TokenStream, GenerateError> {
    let event = rust_ident(&format!("{}Event", source.name))?;
    let observer = rust_ident(&format!("{}Observer", source.name))?;
    let abi = rust_ident(&format!("{}ObserverAbi", source.name))?;
    let adapter = rust_ident(&format!("__{}Observer", source.name))?;
    let exports = rust_ident(&format!("__{}Exports", source.name))?;
    // A sole enum-valued port already is the widget event sum. Reuse that
    // declared message instead of generating a second wrapper enum around it.
    let event_ports = callables.iter().filter_map(|callable| match callable {
        Callable::Port(port) => Some(port),
        _ => None,
    }).collect::<Vec<_>>();
    let direct_event = match event_ports.as_slice() {
        [port] => {
            let payload = resolve(graph, module, &port.message)?;
            std::ptr::eq(payload.module, module)
                && payload.declaration.message
                && payload.declaration.name == format!("{}Event", source.name)
                && matches!(payload.declaration.kind, DeclarationKind::Enum(_))
        }
        _ => false,
    };
    let mut variants = Vec::new();
    let mut methods = Vec::new();
    let mut posts = Vec::new();
    let mut export_fields = Vec::new();
    let mut export_inits = Vec::new();
    let mut export_posts = Vec::new();
    let mut export_binds = Vec::new();
    for callable in callables {
        let Callable::Port(port) = callable else { continue };
        let resolved = resolve(graph, module, &port.message)?;
        let payload = named_rust_type(graph, module, resolved, false)?;
        let method = rust_ident(&port.name)?;
        let receiver = message::receiver_type(graph, module, resolved)?;
        let port_name = Literal::string(&port.name);
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
        let documentation = origin_docs(&port.docs, &port.span);
        if direct_event {
            methods.push(quote! {
                #documentation
                fn #method(&mut self, event: ::neocogi_com::NcMessage<'_, #payload>) -> Result<(), ::neocogi_com::NcStatus> {
                    (self.consume)(event.to_owned()?)
                }
            });
        } else {
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
        export_fields.push(quote! {
            /// At most one component-facing receiver is installed for this event port.
            #method: Option<::neocogi_com::ComPtr<#receiver>>
        });
        export_inits.push(quote! { #method: None });
        export_posts.push(quote! {
            #documentation
            fn #method(&mut self, event: ::neocogi_com::NcMessage<'_, #payload>) -> Result<(), ::neocogi_com::NcStatus> {
                // The form owns visual actions; this relay has one application
                // destination and retains no borrowed event after this call.
                match &mut self.#method {
                    Some(destination) => destination.post(event),
                    None => Ok(()),
                }
            }
        });
        export_binds.push(quote! {
            #port_name => {
                // Query before replacement; a malformed binding preserves the
                // previous application's sole receiver on this port.
                let receiver = destination.add_ref().query::<#receiver>()?;
                self.#method = Some(receiver);
                Ok(())
            }
        });
    }
    let event_declaration = if direct_event {
        quote! {}
    } else {
        quote! {
            /// Owned local snapshots of this producer's outputs, in declared order.
            /// This enum is a Rust convenience and never crosses the COM ABI.
            #[derive(Clone)]
            pub enum #event { #(#variants),* }
        }
    };
    let post_body = if direct_event {
        let method = rust_ident(&event_ports[0].name)?;
        quote! { destination.#method(::neocogi_com::NcMessage::new(self)) }
    } else {
        quote! { match self { #(#posts),* } }
    };
    let post_method = if direct_event { rust_ident("post_to_observer")? } else { rust_ident("post_to")? };
    Ok(quote! {
        #event_declaration

        impl #event {
            /// Owns one callback in the observer allocation for all output kinds.
            /// The callback must own its captures; it cannot borrow a stack frame.
            pub fn observer<F>(consume: F) -> Result<::neocogi_com::ComPtr<#abi>, ::neocogi_com::NcStatus>
            where F: FnMut(Self) -> Result<(), ::neocogi_com::NcStatus> + 'static {
                #adapter::from_value(#adapter { consume }).query()
            }

            /// Creates a fixed observer for a component that exports output names.
            /// The component binds its public destinations through the second
            /// capability. Ordinary channel consumers should use `observer`.
            pub fn exported_observer() -> Result<(
                ::neocogi_com::ComPtr<#abi>,
                ::neocogi_com::ComPtr<::neocogi_com::events::INamedOutputsAbi>,
            ), ::neocogi_com::NcStatus> {
                let owner = #exports::from_value(#exports { #(#export_inits),* });
                Ok((owner.query()?, owner.query()?))
            }

            /// Posts through the observer retained at construction, or does nothing
            /// when the widget was explicitly created without a consumer.
            pub fn #post_method(&self, destination: &mut Option<::neocogi_com::ComPtr<#abi>>) -> Result<(), ::neocogi_com::NcStatus> {
                match destination {
                    Some(destination) => #post_body,
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

        #[::neocogi_com::com_object(interfaces(#observer, ::neocogi_com::events::INamedOutputs))]
        /// Optional component export adapter; never allocated by ordinary observers.
        /// It retains downstream receivers, never the widget or a Rust model pointer.
        struct #exports { #(#export_fields),* }
        impl #observer for #exports { #(#export_posts)* }
        impl ::neocogi_com::events::INamedOutputs for #exports {
            /// Resolves a declared port during setup; delivery uses its field directly.
            fn bind_output(&mut self, name: ::neocogi_com::NcCopyFromBufferOffset<'_, u8>, destination: ::neocogi_com::ComRef<'_, ::neocogi_com::IUnknown>) -> Result<(), ::neocogi_com::NcStatus> {
                if name.offset() != 0 { return Err(::neocogi_com::NC_E_INVALID_ARG); }
                let name = ::core::str::from_utf8(name.as_slice()).map_err(|_| ::neocogi_com::NC_E_INVALID_ARG)?;
                match name {
                    #(#export_binds),*,
                    _ => Err(::neocogi_com::NC_E_NOT_FOUND),
                }
            }
        }
    })
}
