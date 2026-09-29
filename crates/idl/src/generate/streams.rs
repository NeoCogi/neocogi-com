//! Typed receiver adapters for the ordinary IEventQueue<O, E> library contract.
//! Producers receive these objects through construction; no stream projection,
//! attachment method, or widget-specific IDL syntax is generated.
use super::*;

/// Emits only the queue adapter for a closed library receiver specialization.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    match &declaration.specialization {
        Some(specialization) if specialization.template.segments.join(".") == "neocogi.com.events.IEventQueue" => {
            queue(graph, module, declaration, specialization)
        }
        _ => Ok(quote! {}),
    }
}

/// Builds a concrete receiver that validates ownership before calling application mapping code.
fn queue(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    specialization: &crate::Specialization,
) -> Result<TokenStream, GenerateError> {
    let [AbiType::Named(origin), AbiType::Named(message)] = specialization.arguments.as_slice() else {
        return Err(GenerateError::new("IEventQueue requires an interface origin and a declared message type"));
    };
    let resolved_origin = resolve(graph, module, origin)?;
    let DeclarationKind::Interface(origin_interface) = &resolved_origin.declaration.kind else {
        return Err(GenerateError::new("IEventQueue origin must be an interface"));
    };
    // Emit the resolved mode in callback bounds. Leaving a concrete associated
    // projection there creates recursive normalization obligations in com_object.
    let origin_mode = match origin_interface.threading {
        Threading::Local => quote! { ::neocogi_com::ThreadLocal },
        Threading::Shared => quote! { ::neocogi_com::ThreadSafe },
    };
    let origin = named_rust_type(graph, module, resolved_origin, true)?;
    let resolved = resolve(graph, module, message)?;
    if matches!(resolved.declaration.kind, DeclarationKind::Interface(_)) {
        return Err(GenerateError::new("IEventQueue requires a declared value type"));
    }
    let message = named_rust_type(graph, module, resolved, false)?;
    let interface = rust_ident(&declaration.name)?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let adapter = rust_ident(&format!("__{}Receiver", declaration.name))?;
    Ok(quote! {
        impl ::neocogi_com::events::EventQueueInterface for #abi {
            /// Complete owned message acquired from the queue's concrete wire ABI.
            type Object = #origin;
            /// Complete owned payload for this event stream.
            type Event = #message;
            /// Borrows producer-owned storage for this synchronous foreign call.
            fn post(mut destination: ::neocogi_com::ComRef<'_, Self>, origin: ::neocogi_com::ComRef<'_, Self::Object, <Self::Object as ::neocogi_com::ComInterface>::Threading>, message: &Self::Event) -> Result<(), ::neocogi_com::NcStatus> {
                destination.post(origin, ::neocogi_com::NcValueRef::new(message)?)
            }
            /// Stores one typed application mapper inline in its COM receiver allocation.
            fn receiver<F: FnMut(::neocogi_com::WeakComPtr<Self::Object, <Self::Object as ::neocogi_com::ComInterface>::Threading>, Self::Event) -> Result<(), ::neocogi_com::NcStatus> + 'static>(consume: F) -> Result<::neocogi_com::ComPtr<Self>, ::neocogi_com::NcStatus> {
                #adapter::from_value(#adapter { consume }).query()
            }
        }
        #[::neocogi_com::com_object(#interface)]
        /// Owns the parent-local mapper and queue sender; never retains a parent model pointer.
        struct #adapter<F: FnMut(::neocogi_com::WeakComPtr<#origin, #origin_mode>, #message) -> Result<(), ::neocogi_com::NcStatus> + 'static> {
            /// Invoked only after the message's borrowed fields have been acquired.
            consume: F,
        }
        impl<F: FnMut(::neocogi_com::WeakComPtr<#origin, #origin_mode>, #message) -> Result<(), ::neocogi_com::NcStatus> + 'static> #interface for #adapter<F> {
            /// Validates and owns the snapshot before invoking the application mapper.
            fn post(&mut self, origin: ::neocogi_com::ComRef<'_, #origin, #origin_mode>, message: ::neocogi_com::NcValueRef<'_, #message>) -> Result<(), ::neocogi_com::NcStatus> {
                // The foreign producer may release its storage as soon as this call returns.
                (self.consume)(origin.downgrade()?, message.to_owned()?)
            }
        }
    })
}
