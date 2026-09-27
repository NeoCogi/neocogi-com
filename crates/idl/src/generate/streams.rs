//! Library adapters for ordinary generic event contracts.
//!
//! The parser and ABI resolver treat these as ordinary interface templates.
//! This backend layer supplies ergonomic typed Rust helpers for the public
//! neocogi.com.events contracts and enforces one stream on concrete classes.

use std::collections::BTreeSet;
use super::*;

/// Qualified template identity; no method-name scanning is used to discover a stream.
pub(super) const STREAM: &str = "neocogi.com.events.IEventStream";
/// The companion template whose single call delivers one owned message snapshot.
const SINK: &str = "neocogi.com.events.IMessageSink";

/// Emits transport adapters for closed interfaces and aliases for their concrete classes.
pub(super) fn generate(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    if let Some(specialization) = &declaration.specialization {
        match specialization.template.segments.join(".").as_str() {
            SINK => return sink(graph, module, declaration, specialization),
            STREAM => return stream(graph, module, declaration),
            _ => return Ok(quote! {}),
        }
    }
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Ok(quote! {});
    };
    let streams = collect(graph, module, declaration)?;
    if streams.len() > 1 {
        if interface.role == InterfaceRole::Class {
            return Err(GenerateError::new(format!(
                "class {} implements multiple event streams; put every event case in one enum",
                declaration.name
            )));
        }
        return Ok(quote! {});
    }
    let Some(source) = streams.first() else {
        return Ok(quote! {});
    };
    let stream_ty = named_rust_type(graph, module, *source, false)?;
    let mut stream_provider: syn::Path = syn::parse2(stream_ty.clone()).map_err(|error| GenerateError::new(error.to_string()))?;
    let last = stream_provider.segments.last_mut().expect("named stream path is nonempty");
    last.ident = rust_ident(&format!("{}VtblFor", last.ident))?;
    let provider_alias = rust_ident(&format!("{}StreamVtblFor", declaration.name))?;
    let stream_abi = named_rust_type(graph, module, *source, true)?;
    let sink_ty = stream_sink(graph, *source)?;
    let sink_abi = named_rust_type(graph, module, sink_ty, true)?;
    let sink_trait = named_rust_type(graph, module, sink_ty, false)?;
    let mut sink_provider: syn::Path = syn::parse2(sink_trait.clone()).map_err(|error| GenerateError::new(error.to_string()))?;
    let last = sink_provider.segments.last_mut().expect("sink path is nonempty");
    last.ident = rust_ident(&format!("{}VtblFor", last.ident))?;
    let sink_alias = rust_ident(&format!("{}Sink", declaration.name))?;
    let sink_provider_alias = rust_ident(&format!("{}SinkVtblFor", declaration.name))?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    let alias = rust_ident(&format!("{}Stream", declaration.name))?;
    let alias_abi = rust_ident(&format!("{}StreamAbi", declaration.name))?;
    let alias_sink = rust_ident(&format!("{}SinkAbi", declaration.name))?;
    let port = rust_ident(&format!("{}EventPort", declaration.name))?;
    Ok(quote! {
        /// Ordinary implementation trait for this object's complete event stream.
        pub use #stream_ty as #alias;
        /// Typed vtable provider for local implementation lists using the stream alias.
        pub use #stream_provider as #provider_alias;
        /// Closed generic COM projection queried from this object's identity.
        pub type #alias_abi = #stream_abi;
        /// Ordinary typed sink implementation trait for local receiver objects.
        pub use #sink_trait as #sink_alias;
        /// Typed sink vtable provider for explicit local COM implementation lists.
        pub use #sink_provider as #sink_provider_alias;
        /// Exact typed receiver accepted by the one event stream.
        pub type #alias_sink = #sink_abi;
        /// Producer-owned state generated for the class's single lifetime attachment.
        pub type #port = ::neocogi_com::events::EventPort<#sink_abi>;
        impl ::neocogi_com::events::EventSource for #abi {
            /// The one resolved stream shared by this class's projections.
            type Stream = #stream_abi;
        }
    })
}

/// Collects the complete transitive projection set and deduplicates repeated ancestry.
pub(super) fn collect<'a>(
    graph: &'a Generation<'_>,
    module: &'a ResolvedModule,
    declaration: &'a Declaration,
) -> Result<Vec<ResolvedDeclaration<'a>>, GenerateError> {
    /// Walks required projections once while preserving the declaring specialization.
    fn visit<'a>(
        graph: &'a Generation<'_>,
        module: &'a ResolvedModule,
        declaration: &'a Declaration,
        seen: &mut BTreeSet<String>,
        streams: &mut Vec<ResolvedDeclaration<'a>>,
    ) -> Result<(), GenerateError> {
        let key = module.source.name.qualify(&declaration.name);
        if !seen.insert(key) {
            return Ok(());
        }
        let DeclarationKind::Interface(interface) = &declaration.kind else {
            return Ok(());
        };
        if declaration
            .specialization
            .as_ref()
            .is_some_and(|specialization| specialization.template.segments.join(".") == STREAM)
        {
            let identity = module
                .declaration(&declaration.name)
                .ok_or_else(|| GenerateError::new("stream identity is absent"))?;
            if !streams.iter().any(|stream| stream.identity.id == identity.id) {
                streams.push(ResolvedDeclaration { module, declaration, identity });
            }
        }
        for base in &interface.bases {
            let resolved = resolve(graph, module, base)?;
            visit(graph, resolved.module, resolved.declaration, seen, streams)?;
        }
        Ok(())
    }
    let mut streams = Vec::new();
    visit(graph, module, declaration, &mut BTreeSet::new(), &mut streams)?;
    Ok(streams)
}

/// Resolves the concrete sink from the stream's checked attach method signature.
fn stream_sink<'a>(graph: &'a Generation<'_>, stream: ResolvedDeclaration<'a>) -> Result<ResolvedDeclaration<'a>, GenerateError> {
    let DeclarationKind::Interface(interface) = &stream.declaration.kind else {
        unreachable!("stream is an interface")
    };
    let [Callable::Method(method)] = interface.callables.as_slice() else {
        return Err(GenerateError::new("IEventStream must declare exactly one attach method"));
    };
    let [parameter] = method.parameters.as_slice() else {
        return Err(GenerateError::new("IEventStream.attach requires one typed sink"));
    };
    let AbiType::Borrowed(value) = &parameter.ty else {
        return Err(GenerateError::new("IEventStream.attach must borrow its sink"));
    };
    let AbiType::Named(name) = value.as_ref() else {
        return Err(GenerateError::new("IEventStream.attach requires a named sink"));
    };
    if method.name != "attach" {
        return Err(GenerateError::new("IEventStream method must be attach"));
    }
    resolve(graph, stream.module, name)
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

/// Implements the common typed client operation using this specialization's concrete ABI.
fn stream(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let source = ResolvedDeclaration {
        module,
        declaration,
        identity: module
            .declaration(&declaration.name)
            .ok_or_else(|| GenerateError::new("stream identity is absent"))?,
    };
    let sink = named_rust_type(graph, module, stream_sink(graph, source)?, true)?;
    let abi = rust_ident(&format!("{}Abi", declaration.name))?;
    Ok(quote! {
        impl ::neocogi_com::events::EventStreamInterface for #abi {
            /// Exact sink specialization required by the stream's attach method.
            type Sink = #sink;
            /// Crosses the typed COM gate; the producer enforces its lifetime state.
            fn attach(mut source: ::neocogi_com::ComRef<'_, Self>, consumer: ::neocogi_com::ComRef<'_, Self::Sink>) -> Result<(), ::neocogi_com::NcStatus> {
                source.attach(consumer)
            }
        }
    })
}
