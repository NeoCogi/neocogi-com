//! Static type associations for the ordinary IEventQueue<O, E> library contract.
//! Producers receive these objects through construction; no stream projection,
//! attachment method, or widget-specific IDL syntax is generated.
use super::*;

/// Associates a closed queue specialization with its exact origin and payload.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    match &declaration.specialization {
        Some(specialization) if specialization.template.segments.join(".") == "neocogi.com.events.IEventQueue" => {
            queue(graph, module, declaration, specialization)
        }
        _ => Ok(quote! {}),
    }
}

/// Emits producer-side typed posting access; application queue implementations stay ordinary COM objects.
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
    if !matches!(resolved_origin.declaration.kind, DeclarationKind::Interface(_)) {
        return Err(GenerateError::new("IEventQueue origin must be an interface"));
    }
    let origin = named_rust_type(graph, module, resolved_origin, true)?;
    let resolved = resolve(graph, module, message)?;
    if matches!(resolved.declaration.kind, DeclarationKind::Interface(_)) {
        return Err(GenerateError::new("IEventQueue requires a declared value type"));
    }
    let message = named_rust_type(graph, module, resolved, false)?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    Ok(quote! {
        impl ::neocogi_com::events::EventQueueInterface for #abi {
            /// Exact public producer interface borrowed for the duration of posting.
            type Object = #origin;
            /// Complete owned payload for this event stream.
            type Event = #message;
            /// Borrows producer-owned storage for this synchronous foreign call.
            fn post(mut destination: ::neocogi_com::ComRef<'_, Self>, origin: ::neocogi_com::ComRef<'_, Self::Object, <Self::Object as ::neocogi_com::ComInterface>::Threading>, message: &Self::Event) -> Result<(), ::neocogi_com::NcStatus> {
                destination.post(origin, ::neocogi_com::NcValueRef::new(message)?)
            }
        }
    })
}
