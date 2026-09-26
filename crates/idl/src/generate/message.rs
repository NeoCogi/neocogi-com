//! Concrete owned message generation and one typed borrowed delivery operation.

use super::*;
use quote::format_ident;

/// Emits owned data, a C wire record, checked conversion and one concrete receiver.
pub(super) fn generate_message(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    identity: &crate::DeclarationIdentity,
) -> Result<String, GenerateError> {
    let name = rust_ident(&declaration.name)?;
    let wire = format_ident!("__{name}Wire");
    let union = format_ident!("__{name}Payload");
    let receiver = format_ident!("__{name}Receiver");
    let receiver_abi = format_ident!("__{name}ReceiverAbi");
    let adapter = format_ident!("__{name}Mapping");
    let declaration_docs = origin_docs(&declaration.docs, &declaration.span);
    let signature = signature_literal(identity.abi_signature);
    let receiver_name = format!("{}.{}.__receiver", module.source.name, declaration.name);
    let id = Literal::string(&neocogi_com_core::guid_from_qualified_name(&receiver_name).to_string());
    let qualified_name = Literal::string(&receiver_name);
    let mut cases = Vec::new();
    match &declaration.kind {
        DeclarationKind::Struct(record) => cases.push((None, 0i64, VariantFields::Named(record.fields.clone()), Vec::new(), declaration.span.clone())),
        DeclarationKind::Enum(enumeration) => {
            let mut previous = -1;
            for variant in &enumeration.variants {
                let tag = variant.discriminant.unwrap_or(previous + 1);
                previous = tag;
                cases.push((
                    Some(rust_ident(&variant.name)?),
                    tag,
                    variant.fields.clone(),
                    variant.docs.clone(),
                    variant.span.clone(),
                ));
            }
        }
        _ => return Err(GenerateError::new("messages require structs or enums")),
    }
    let mut declarations = Vec::new();
    let mut wire_cases = Vec::new();
    let mut members = Vec::new();
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    let mut comparable = true;
    for (index, (variant, tag, fields, documentation, origin)) in cases.iter().enumerate() {
        let case = format_ident!("__{name}Case{index}");
        let member = format_ident!("case_{index}");
        let case_docs = origin_docs(documentation, origin);
        let values = match fields {
            VariantFields::Unit => Vec::new(),
            VariantFields::Named(fields) => fields
                .iter()
                .map(|field| Ok((rust_ident(&field.name)?, &field.ty, origin_docs(&field.docs, &field.span), &field.span)))
                .collect::<Result<Vec<_>, GenerateError>>()?,
            VariantFields::Tuple(fields) => fields
                .iter()
                .enumerate()
                .map(|(index, ty)| (format_ident!("field_{index}"), ty, quote! { #[doc = "Owned message field."] }, origin))
                .collect(),
        };
        let mut native = Vec::new();
        let mut raw = Vec::new();
        let mut encodings = Vec::new();
        let mut decodings = Vec::new();
        let mut bindings = Vec::new();
        for (field, ty, field_docs, field_origin) in values {
            let (owned, abi, encoded, decoded, equality) = field_codec(graph, module, ty, &field)?;
            comparable &= equality;
            native.push(match fields {
                VariantFields::Tuple(_) => quote! { #owned },
                _ if variant.is_none() => quote! { #field_docs pub #field: #owned },
                _ => quote! { #field_docs #field: #owned },
            });
            let wire_origin = origin_docs(&[], field_origin);
            raw.push(quote! { /// Borrowed field, valid only during this message's COM call.
            #wire_origin pub #field: #abi });
            encodings.push(quote! { #field: #encoded });
            decodings.push(decoded);
            bindings.push(field);
        }
        let constructor = match variant {
            Some(variant) => quote! { Self::#variant },
            None => quote! { Self },
        };
        let (shape, pattern, value) = match fields {
            VariantFields::Unit => (quote! {}, constructor.clone(), constructor.clone()),
            VariantFields::Tuple(_) => (
                quote! { (#(#native),*) },
                quote! { #constructor(#(#bindings),*) },
                quote! { #constructor(#(#decodings),*) },
            ),
            VariantFields::Named(_) => (
                quote! { { #(#native),* } },
                quote! { #constructor { #(#bindings),* } },
                quote! { #constructor { #(#bindings: #decodings),* } },
            ),
        };
        declarations.push(match variant {
            Some(variant) => quote! { #case_docs #variant #shape },
            None => shape,
        });
        if bindings.is_empty() {
            raw.push(quote! { /// Initialized placeholder for a payload with no fields.
            pub empty: u8 });
            encodings.push(quote! { empty: 0 });
        }
        let case_origin = origin_docs(&[], origin);
        wire_cases.push(quote! {
            #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
            /// Generated payload for one concrete message case.
            #case_origin pub struct #case { #(#raw),* }
        });
        members.push(quote! { /// Payload selected by its declaration-order tag.
        #case_origin pub #member: #case });
        encode.push(quote! { #pattern => #wire { tag: #tag, payload: #union { #member: #case { #(#encodings),* } } } });
        let read_payload = (!bindings.is_empty()).then(|| {
            quote! {
                // SAFETY: only the member selected by this validated tag is read.
                let raw = unsafe { wire.payload.#member };
            }
        });
        decode.push(quote! { #tag => {
            // SAFETY: the validated tag selects this initialized member. Borrowed
            // data is checked and acquired before the owned message is returned.
            #read_payload
            Ok(#value)
        } });
    }
    let equality = comparable.then(|| quote! { #[derive(Debug, PartialEq)] });
    let mut plain = declaration.clone();
    plain.message = false;
    // A nested message uses its borrowed wire inside this message. Even when
    // its current fields are copyable, keep the enclosing declaration on the
    // owned-message path so future nested ownership cannot become plain ABI.
    let copyable_field = |ty: &AbiType| {
        !matches!(ty, AbiType::Named(path) if resolve(graph, module, path).is_ok_and(|nested| nested.declaration.message))
            && value_type(graph, module, ty).is_ok()
    };
    let copyable = cases.iter().all(|(_, _, fields, _, _)| match fields {
        VariantFields::Unit => true,
        VariantFields::Tuple(fields) => fields.iter().all(&copyable_field),
        VariantFields::Named(fields) => fields.iter().all(|field| copyable_field(&field.ty)),
    });
    let data = if copyable {
        // A plain message keeps its existing ordinary ABI representation as well
        // as gaining generated posting. No wrapper record is needed around it.
        let index = module
            .source
            .declarations
            .iter()
            .position(|item| item.name == declaration.name)
            .expect("resolved declaration exists");
        generate_declaration(graph, module, &plain, index)?
            .parse::<TokenStream>()
            .map_err(|error| GenerateError::new(error.to_string()))?
    } else {
        match declaration.kind {
            DeclarationKind::Struct(_) => quote! { #declaration_docs #[derive(Clone)] #equality pub struct #name #(#declarations)* },
            _ => quote! { #declaration_docs #[derive(Clone)] #equality pub enum #name { #(#declarations),* } },
        }
    };
    format_rust(quote! {
        #data
        #(#wire_cases)*
        #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
        /// Exact payload alternatives, inspected only after validating the message tag.
        pub union #union { #(#members),* }
        #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
        /// Call-scoped C message representation; contains no Rust ownership or drop glue.
        pub struct #wire {
            /// Declared message case; records use zero.
            pub tag: i64,
            /// Only the member selected by tag is readable.
            pub payload: #union,
        }
        // SAFETY: the generated C fields borrow from this exact concrete message;
        // decoding validates the active alternative and acquires every owned field.
        unsafe impl ::neocogi_com::ComMessage for #name {
            type Wire = #wire;
            const SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(#signature);
            fn encode(&self) -> Self::Wire { match self { #(#encode),* } }
            unsafe fn decode_with_budget(wire: Self::Wire, budget: &mut ::neocogi_com::MessageDecodeBudget) -> Result<Self, ::neocogi_com::NcStatus> {
                budget.record()?;
                match wire.tag { #(#decode),*, _ => Err(::neocogi_com::NC_E_INVALID_TAG) }
            }
        }
        #[doc(hidden)]
        #[::neocogi_com::com_interface(assigned_guid = #id, name = #qualified_name, assigned_signature = #signature)]
        /// One concrete message delivery across COM; no application type crosses this boundary.
        pub trait #receiver {
            /// Acquires the message before its producer's borrowed storage expires.
            fn post(&mut self, message: ::neocogi_com::NcMessage<'_, #name>) -> Result<(), ::neocogi_com::NcStatus>;
        }
        impl #name {
            /// Posts to the installed destination, borrowing this value for the COM call only.
            /// An absent destination is intentionally a successful no-op.
            pub fn post_to(&self, destination: &mut Option<::neocogi_com::ComPtr<#receiver_abi>>) -> Result<(), ::neocogi_com::NcStatus> {
                match destination {
                    Some(receiver) => receiver.post(::neocogi_com::NcMessage::new(self)),
                    None => Ok(()),
                }
            }
            #[doc(hidden)]
            /// Wraps a local, concretely typed callback in the one checked COM receiver.
            pub fn __receiver(consume: self::alloc::boxed::Box<dyn FnMut(Self) -> Result<(), ::neocogi_com::NcStatus>>) -> Result<::neocogi_com::ComPtr<#receiver_abi>, ::neocogi_com::NcStatus> {
                #adapter::from_value(#adapter { consume }).query()
            }
        }
        impl ::neocogi_com::ComEvent for #name {
            /// Keeps the concrete payload and callback local while exposing a COM capability.
            fn make_receiver(consume: self::alloc::boxed::Box<dyn FnMut(Self) -> Result<(), ::neocogi_com::NcStatus>>)
                -> Result<::neocogi_com::ComPtr<::neocogi_com::IUnknown>, ::neocogi_com::NcStatus>
            {
                Self::__receiver(consume)?.query()
            }
        }
        #[::neocogi_com::com_object(interfaces(#receiver))]
        /// Owns only a typed callback; no registration or connection object is allocated.
        struct #adapter {
            /// Decoded messages reach this local callback before producer storage expires.
            consume: self::alloc::boxed::Box<dyn FnMut(#name) -> Result<(), ::neocogi_com::NcStatus>>,
        }
        impl #receiver for #adapter {
            /// Validate and acquire borrowed fields before mapping or queueing anything.
            fn post(&mut self, message: ::neocogi_com::NcMessage<'_, #name>) -> Result<(), ::neocogi_com::NcStatus> {
                (self.consume)(message.to_owned()?)
            }
        }

    })
}

/// Selects owned storage, exact wire fields, and checked conversion for one message field.
fn field_codec(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    ty: &AbiType,
    field: &Ident,
) -> Result<(TokenStream, TokenStream, TokenStream, TokenStream, bool), GenerateError> {
    // A nested message is an inline C wire record, not a Rust value passed
    // through the ABI. The outer borrow keeps every nested pointer live until
    // recursive decoding has acquired the complete owned value.
    if let AbiType::Named(path) = ty {
        let nested = resolve(graph, module, path)?;
        if nested.declaration.message {
            let owned = named_rust_type(graph, module, nested, false)?;
            return Ok((
                owned.clone(),
                quote! { <#owned as ::neocogi_com::ComMessage>::Wire },
                quote! { <#owned as ::neocogi_com::ComMessage>::encode(#field) },
                quote! {
                    // SAFETY: the outer call guarantees the nested inline wire and
                    // all its borrowed backing storage remain live for this decode.
                    unsafe { <#owned as ::neocogi_com::ComMessage>::decode_with_budget(raw.#field, budget) }?
                },
                false,
            ));
        }
    }
    match ty {
        AbiType::String | AbiType::Slice(_) => {
            let (element, owned, bytes, acquire) = match ty {
                AbiType::String => (
                    quote! { u8 },
                    quote! { self::alloc::string::String },
                    quote! { #field.as_bytes() },
                    quote! { ::neocogi_com::copy_message_text(buffer, budget)? },
                ),
                AbiType::Slice(element) if matches!(element.as_ref(), AbiType::Scalar(name) if name == "bool") => (
                    quote! { u8 },
                    quote! { self::alloc::vec::Vec<bool> },
                    quote! { #field.as_slice() },
                    quote! { ::neocogi_com::copy_message_bools(buffer, budget)? },
                ),
                AbiType::Slice(element) => {
                    let element = value_type(graph, module, element)?;
                    (
                        element.clone(),
                        quote! { self::alloc::vec::Vec<#element> },
                        quote! { #field.as_slice() },
                        quote! { ::neocogi_com::copy_message_slice(buffer, budget)? },
                    )
                }
                _ => unreachable!(),
            };
            Ok((
                owned,
                quote! { ::neocogi_com::NcMessageSlice<#element> },
                if matches!(ty, AbiType::Slice(element) if matches!(element.as_ref(), AbiType::Scalar(name) if name == "bool")) {
                    quote! { ::neocogi_com::NcMessageSlice::from_bools(#bytes) }
                } else {
                    quote! { ::neocogi_com::NcMessageSlice::new(#bytes) }
                },
                quote! {{
                    // SAFETY: the raw caller guarantees readable, initialized backing storage.
                    let buffer = unsafe { raw.#field.as_slice_bounded(budget) }?;
                    #acquire
                }},
                true,
            ))
        }
        AbiType::Named(path) if matches!(resolve(graph, module, path)?.declaration.kind, DeclarationKind::Interface(_)) => {
            let abi = named_rust_type(graph, module, resolve(graph, module, path)?, true)?;
            let owned = object_reference_type(graph, module, path, true)?;
            let borrowed = object_reference_type(graph, module, path, false)?;
            Ok((
                owned,
                quote! { *mut #abi },
                quote! { #field.as_mut_ptr() },
                quote! {
                    // SAFETY: the raw caller keeps this object live; its full identity is checked.
                    unsafe { <#borrowed>::__nc_from_raw_borrowed_checked(raw.#field) }?.add_ref()
                },
                false,
            ))
        }
        AbiType::Scalar(name) if name == "bool" => Ok((
            quote! { bool },
            quote! { u8 },
            quote! { u8::from(*#field) },
            quote! { match raw.#field { 0 => false, 1 => true, _ => return Err(::neocogi_com::NC_E_INVALID_ARG) } },
            true,
        )),
        _ => {
            let abi = value_type(graph, module, ty)?;
            match ty {
                AbiType::Named(path) => {
                    let resolved = resolve(graph, module, path)?;
                    let owned = named_rust_type(graph, module, resolved, false)?;
                    match &resolved.declaration.kind {
                        DeclarationKind::Enum(enumeration) if enum_has_payload(enumeration) || resolved.declaration.message => Ok((
                            owned.clone(),
                            abi,
                            quote! { (*#field).into() },
                            quote! { #owned::try_from(raw.#field).map_err(|_| ::neocogi_com::NC_E_INVALID_ARG)? },
                            true,
                        )),
                        _ => Ok((owned, abi, quote! { *#field }, quote! { raw.#field }, true)),
                    }
                }
                _ => Ok((abi.clone(), abi, quote! { *#field }, quote! { raw.#field }, true)),
            }
        }
    }
}
