//! Typed receiver adapters for the ordinary IMessageSink<E> library contract.
//! Producers receive these objects through construction; no stream projection,
//! attachment method, or widget-specific IDL syntax is generated.
use super::*;

/// Emits only the sink adapter for a closed library receiver specialization.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    match &declaration.specialization {
        Some(specialization) if specialization.template.segments.join(".") == "neocogi.com.events.IMessageSink" => {
            sink(graph, module, declaration, specialization)
        }
        _ => Ok(quote! {}),
    }
}

/// Builds a concrete receiver that validates ownership before calling application mapping code.
fn sink(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    specialization: &crate::Specialization,
) -> Result<TokenStream, GenerateError> {
    let [AbiType::Named(message)] = specialization.arguments.as_slice() else {
        return Err(GenerateError::new("IMessageSink requires one declared message type"));
    };
    let resolved = resolve(graph, module, message)?;
    if matches!(resolved.declaration.kind, DeclarationKind::Interface(_)) {
        return Err(GenerateError::new("IMessageSink requires a declared value type"));
    }
    let message = named_rust_type(graph, module, resolved, false)?;
    let interface = rust_ident(&declaration.name)?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let adapter = rust_ident(&format!("__{}Receiver", declaration.name))?;
    Ok(quote! {
        impl ::neocogi_com::events::MessageSinkInterface for #abi {
            /// Complete owned message acquired from the sink's concrete wire ABI.
            type Message = #message;
            /// Borrows producer-owned storage for this synchronous foreign call.
            fn post(mut destination: ::neocogi_com::ComRef<'_, Self>, message: &Self::Message) -> Result<(), ::neocogi_com::NcStatus> {
                destination.post(::neocogi_com::NcValueRef::new(message)?)
            }
            /// Stores one typed application mapper inline in its COM receiver allocation.
            fn receiver<F: FnMut(Self::Message) -> Result<(), ::neocogi_com::NcStatus> + 'static>(consume: F) -> Result<::neocogi_com::ComPtr<Self>, ::neocogi_com::NcStatus> {
                #adapter::from_value(#adapter { consume }).query()
            }
        }
        #[::neocogi_com::com_object(#interface)]
        /// Owns the parent-local mapper and queue sender; never retains a parent model pointer.
        struct #adapter<F: FnMut(#message) -> Result<(), ::neocogi_com::NcStatus> + 'static> {
            /// Invoked only after the message's borrowed fields have been acquired.
            consume: F,
        }
        impl<F: FnMut(#message) -> Result<(), ::neocogi_com::NcStatus> + 'static> #interface for #adapter<F> {
            /// Validates and owns the snapshot before invoking the application mapper.
            fn post(&mut self, message: ::neocogi_com::NcValueRef<'_, #message>) -> Result<(), ::neocogi_com::NcStatus> {
                // The foreign producer may release its storage as soon as this call returns.
                (self.consume)(message.to_owned()?)
            }
        }
    })
}
