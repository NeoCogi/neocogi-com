//! Validation and ABI lowering for `#[com_interface]` methods.
//!
//! The parser accepts a deliberately small Rust surface and lowers it into the
//! normalized model in `model.rs`. Argument and return lowering encode ownership
//! explicitly: plain values cross by value/out pointer, `ComRef` is borrowed,
//! `ComPtr` returns transfer ownership, and bounded buffers carry call-scoped
//! pointers. Later generators consume this model without reinterpreting syntax.

use quote::quote;
use syn::{
    Attribute, FnArg, Ident, ItemTrait, Pat, Result, ReturnType, TraitItem, TraitItemFn, Type,
};

use crate::model::{AbiArg, AbiArgKind, AbiConstructor, AbiMethod, AbiReturn, AbiReturnKind};
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_type};
use crate::util::push_error;

/// Validates one trait method and lowers it into the shared ABI model.
fn parse_abi_method(method: &TraitItemFn) -> Result<AbiMethod> {
    // Accumulate independent signature errors so one macro invocation reports
    // all actionable problems instead of forcing a compile/fix cycle per item.
    let mut errors = None;
    validate_method_modifiers(method, &mut errors);
    validate_receiver(method, &mut errors);

    let args = match parse_abi_args(method) {
        Ok(args) => args,
        Err(err) => {
            push_error(&mut errors, err);
            Vec::new()
        }
    };
    let ok = match parse_result_ok_type(method).and_then(lower_abi_return) {
        Ok(ok) => ok,
        Err(err) => {
            push_error(&mut errors, err);
            AbiReturn {
                rust_ty: syn::parse_quote! { () },
                abi_ty: syn::parse_quote! { () },
                kind: AbiReturnKind::Plain,
            }
        }
    };

    if let Some(errors) = errors {
        return Err(errors);
    }

    Ok(AbiMethod {
        ident: method.sig.ident.clone(),
        doc_attrs: method_doc_attrs(&method.attrs),
        args,
        ok,
    })
}

/// Validates and lowers the one static constructor on a callable class trait.
pub(crate) fn parse_class_constructor(method: &TraitItemFn) -> Result<AbiConstructor> {
    let mut errors = None;
    validate_method_modifiers(method, &mut errors);

    if method.sig.inputs.iter().any(|input| matches!(input, FnArg::Receiver(_))) {
        push_error(
            &mut errors,
            syn::Error::new_spanned(&method.sig.inputs, "class constructors are static functions and cannot take `self`"),
        );
    }
    let args = match parse_constructor_args(method) {
        Ok(args) => args,
        Err(err) => {
            push_error(&mut errors, err);
            Vec::new()
        }
    };
    match parse_result_ok_type(method) {
        Ok(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("Self") => {}
        Ok(ty) => push_error(
            &mut errors,
            syn::Error::new_spanned(ty, "class constructors must return `Result<Self, NcStatus>`"),
        ),
        Err(err) => push_error(&mut errors, err),
    }

    for arg in &args {
        if matches!(arg.kind, AbiArgKind::CopyToBufferOffset { .. }) {
            push_error(
                &mut errors,
                syn::Error::new_spanned(&arg.rust_ty, "class constructors cannot take writable output buffers"),
            );
        }
    }

    if let Some(errors) = errors {
        return Err(errors);
    }
    Ok(AbiConstructor {
        ident: method.sig.ident.clone(),
        doc_attrs: method_doc_attrs(&method.attrs),
        args,
    })
}

/// Lowers every named static constructor parameter into the shared ABI model.
fn parse_constructor_args(method: &TraitItemFn) -> Result<Vec<AbiArg>> {
    let mut errors = None;
    let mut args = Vec::new();
    for arg in &method.sig.inputs {
        match arg {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat) if pat.by_ref.is_none() && pat.mutability.is_none() && pat.subpat.is_none() => match lower_abi_arg(typed.ty.as_ref().clone()) {
                    Ok((rust_ty, abi_ty, kind)) => args.push(AbiArg {
                        ident: pat.ident.clone(),
                        rust_ty,
                        abi_ty,
                        kind,
                    }),
                    Err(err) => push_error(&mut errors, err),
                },
                _ => push_error(
                    &mut errors,
                    syn::Error::new_spanned(&typed.pat, "class constructor arguments must be plain identifiers"),
                ),
            },
            FnArg::Receiver(receiver) => push_error(&mut errors, syn::Error::new_spanned(receiver, "class constructors cannot take `self`")),
        }
    }
    if let Some(errors) = errors { Err(errors) } else { Ok(args) }
}

/// Rejects Rust function features that have no stable C ABI lowering.
fn validate_method_modifiers(method: &TraitItemFn, errors: &mut Option<syn::Error>) {
    if let Some(default) = &method.default {
        push_error(errors, syn::Error::new_spanned(default, "COM methods cannot have default bodies"));
    }
    if let Some(constness) = &method.sig.constness {
        push_error(errors, syn::Error::new_spanned(constness, "COM methods cannot be const"));
    }
    if let Some(asyncness) = &method.sig.asyncness {
        push_error(errors, syn::Error::new_spanned(asyncness, "COM methods cannot be async"));
    }
    if let Some(unsafety) = &method.sig.unsafety {
        push_error(errors, syn::Error::new_spanned(unsafety, "COM methods cannot be unsafe"));
    }
    if let Some(abi) = &method.sig.abi {
        push_error(errors, syn::Error::new_spanned(abi, "COM methods cannot specify an extern ABI"));
    }
    if method.sig.variadic.is_some() {
        push_error(errors, syn::Error::new_spanned(&method.sig.variadic, "COM methods cannot be variadic"));
    }
    if !method.sig.generics.params.is_empty() {
        push_error(errors, syn::Error::new_spanned(&method.sig.generics, "COM methods cannot be generic"));
    }
    if let Some(where_clause) = &method.sig.generics.where_clause {
        push_error(errors, syn::Error::new_spanned(where_clause, "COM methods cannot have where clauses"));
    }
}

/// Accepts only trait methods and delegates their complete ABI validation.
pub(crate) fn parse_abi_trait_item(item: &TraitItem) -> Result<AbiMethod> {
    match item {
        TraitItem::Fn(method) => parse_abi_method(method),
        _ => Err(syn::Error::new_spanned(
            item,
            "COM interfaces may only contain methods",
        )),
    }
}

/// Rejects trait-level features that cannot be represented by one C vtable.
pub(crate) fn validate_interface_trait(input: &ItemTrait, errors: &mut Option<syn::Error>) {
    validate_com_trait(input, errors);
    if input.supertraits.len() > 1 {
        push_error(
            errors,
            syn::Error::new_spanned(&input.supertraits, "COM interfaces support exactly one generated base interface"),
        );
    }
}

/// Rejects trait features unsupported by both interfaces and callable classes.
pub(crate) fn validate_class_contract_trait(input: &ItemTrait, errors: &mut Option<syn::Error>) {
    validate_com_trait(input, errors);
}

/// Applies the trait-shape rules shared by every generated COM vtable.
fn validate_com_trait(input: &ItemTrait, errors: &mut Option<syn::Error>) {
    if let Some(unsafety) = &input.unsafety {
        push_error(
            errors,
            syn::Error::new_spanned(unsafety, "COM interfaces cannot be unsafe"),
        );
    }
    if let Some(auto_token) = &input.auto_token {
        push_error(
            errors,
            syn::Error::new_spanned(auto_token, "COM interfaces cannot be auto traits"),
        );
    }
    if !input.generics.params.is_empty() {
        push_error(
            errors,
            syn::Error::new_spanned(&input.generics, "COM interfaces cannot be generic"),
        );
    }
    if let Some(where_clause) = &input.generics.where_clause {
        push_error(
            errors,
            syn::Error::new_spanned(where_clause, "COM interfaces cannot have where clauses"),
        );
    }
}

/// Requires exactly one ordinary shared `&self` receiver.
fn validate_receiver(method: &TraitItemFn, errors: &mut Option<syn::Error>) {
    match method.sig.inputs.first() {
        Some(FnArg::Receiver(receiver)) => {
            let has_explicit_lifetime = receiver
                .reference
                .as_ref()
                .and_then(|(_, lifetime)| lifetime.as_ref())
                .is_some();
            if receiver.reference.is_none()
                || receiver.mutability.is_some()
                || receiver.colon_token.is_some()
                || has_explicit_lifetime
            {
                push_error(
                    errors,
                    syn::Error::new_spanned(receiver, "COM methods currently require `&self`"),
                );
            }
        }
        Some(arg) => push_error(
            errors,
            syn::Error::new_spanned(arg, "COM methods must take `&self` as the first parameter"),
        ),
        None => push_error(
            errors,
            syn::Error::new_spanned(&method.sig.ident, "COM methods must take `&self`"),
        ),
    }
}

/// Lowers named method parameters after skipping the validated receiver.
fn parse_abi_args(method: &TraitItemFn) -> Result<Vec<AbiArg>> {
    let mut errors = None;
    let mut args = Vec::new();

    // The receiver was validated separately; only logical ABI parameters are
    // represented in descriptor metadata and generated vtable arguments.
    for arg in method.sig.inputs.iter().skip(1) {
        match arg {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat)
                    if pat.by_ref.is_none() && pat.mutability.is_none() && pat.subpat.is_none() =>
                {
                    match lower_abi_arg(typed.ty.as_ref().clone()) {
                        Ok((rust_ty, abi_ty, kind)) => args.push(AbiArg {
                            ident: pat.ident.clone(),
                            rust_ty,
                            abi_ty,
                            kind,
                        }),
                        Err(err) => push_error(&mut errors, err),
                    }
                }
                _ => push_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        &typed.pat,
                        "COM method arguments must be plain identifiers",
                    ),
                ),
            },
            FnArg::Receiver(receiver) => push_error(
                &mut errors,
                syn::Error::new_spanned(receiver, "COM methods can only have one `self` receiver"),
            ),
        }
    }

    if let Some(errors) = errors {
        Err(errors)
    } else {
        Ok(args)
    }
}

/// Extracts `T` from the required `Result<T, NcStatus>` return shape.
fn parse_result_ok_type(method: &TraitItemFn) -> Result<Type> {
    // `Result<T, NcStatus>` lowers to an ABI status plus either an out pointer
    // for non-void success values or no out pointer for `Result<(), NcStatus>`.
    // Validate the complete shape before generation so unsupported signatures
    // fail at the user's method declaration instead of inside generated tokens.
    let ty = match &method.sig.output {
        ReturnType::Type(_, ty) => ty.as_ref(),
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "COM methods must return `Result<T, nc_com::NcStatus>`",
            ));
        }
    };

    let path = match ty {
        Type::Path(path) => path,
        _ => {
            return Err(syn::Error::new_spanned(
                ty,
                "COM methods must return `Result<T, nc_com::NcStatus>`",
            ));
        }
    };

    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "COM methods must return `Result<T, nc_com::NcStatus>`",
        ));
    };
    if segment.ident != "Result" {
        return Err(syn::Error::new_spanned(
            ty,
            "COM methods must return `Result<T, nc_com::NcStatus>`",
        ));
    }

    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            &segment.arguments,
            "COM methods must return `Result<T, nc_com::NcStatus>`",
        ));
    };
    let generic_args = args.args.iter().collect::<Vec<_>>();
    if generic_args.len() != 2 {
        return Err(syn::Error::new_spanned(
            &segment.arguments,
            "COM methods must return `Result<T, nc_com::NcStatus>`",
        ));
    }

    let ok = match generic_args[0] {
        syn::GenericArgument::Type(ty) => ty.clone(),
        arg => {
            return Err(syn::Error::new_spanned(
                arg,
                "COM method success type must be a type",
            ));
        }
    };
    let err = match generic_args[1] {
        syn::GenericArgument::Type(ty) => ty,
        arg => {
            return Err(syn::Error::new_spanned(
                arg,
                "COM method error type must be `nc_com::NcStatus`",
            ));
        }
    };
    if !is_nc_status_type(err) {
        return Err(syn::Error::new_spanned(
            err,
            "COM method error type must be `nc_com::NcStatus`",
        ));
    }

    Ok(ok)
}

/// Selects the raw parameter type and ownership rule for one Rust argument.
fn lower_abi_arg(ty: Type) -> Result<(Type, Type, AbiArgKind)> {
    // Recognize ownership-bearing wrappers before the plain-value fallback.
    // Ordering matters because raw ABI pointer types alone cannot communicate
    // whether an interface or buffer is borrowed, mutable, or transferred.
    if let Some(com_ref) = parse_com_ref_type(&ty)? {
        let abi_ty = raw_interface_pointer_ty(&com_ref.interface_ty);
        return Ok((
            ty,
            abi_ty,
            AbiArgKind::ComRef {
                interface_ty: Box::new(com_ref.interface_ty),
                mode_ty: com_ref.mode_ty.map(Box::new),
            },
        ));
    }

    if let Some(buffer) = parse_copy_from_buffer_offset_type(&ty)? {
        let element_ty = buffer.element_ty;
        validate_plain_abi_type(&element_ty, PlainAbiTypeContext::BufferElement)?;
        let abi_ty = syn::parse_quote! { *const #element_ty };
        return Ok((
            ty,
            abi_ty,
            AbiArgKind::CopyFromBufferOffset {
                element_ty: Box::new(element_ty),
            },
        ));
    }

    if let Some(buffer) = parse_copy_to_buffer_offset_type(&ty)? {
        let element_ty = buffer.element_ty;
        validate_plain_abi_type(&element_ty, PlainAbiTypeContext::BufferElement)?;
        let abi_ty = syn::parse_quote! { *mut #element_ty };
        return Ok((
            ty,
            abi_ty,
            AbiArgKind::CopyToBufferOffset {
                element_ty: Box::new(element_ty),
            },
        ));
    }

    if parse_com_ptr_type(&ty)?.is_some() {
        return Err(syn::Error::new_spanned(
            ty,
            "COM method arguments cannot take `ComPtr`; use `ComRef<'_, InterfaceAbi>` for borrowed interface parameters and call `add_ref()` if the callee must retain the object",
        ));
    }

    validate_plain_abi_type(&ty, PlainAbiTypeContext::MethodArgument)?;

    Ok((ty.clone(), ty, AbiArgKind::Plain))
}

/// Selects status-only, plain out-value, or owned-interface return lowering.
fn lower_abi_return(ty: Type) -> Result<AbiReturn> {
    // `()` has no out slot, owned COM pointers transfer one reference, and all
    // remaining accepted types use a caller-provided plain-value out pointer.
    if is_unit_type(&ty) {
        return Ok(AbiReturn {
            rust_ty: ty,
            abi_ty: syn::parse_quote! { () },
            kind: AbiReturnKind::Void,
        });
    }

    if let Some(com_ptr) = parse_com_ptr_type(&ty)? {
        let abi_ty = raw_interface_pointer_ty(&com_ptr.interface_ty);
        return Ok(AbiReturn {
            rust_ty: ty,
            abi_ty,
            kind: AbiReturnKind::ComPtr {
                interface_ty: Box::new(com_ptr.interface_ty),
                mode_ty: com_ptr.mode_ty.map(Box::new),
            },
        });
    }

    if parse_com_ref_type(&ty)?.is_some() {
        return Err(syn::Error::new_spanned(
            ty,
            "COM methods cannot return `ComRef`; return `ComPtr<InterfaceAbi>` to transfer an owned interface reference",
        ));
    }

    validate_plain_abi_type(&ty, PlainAbiTypeContext::MethodReturn)?;

    Ok(AbiReturn {
        rust_ty: ty.clone(),
        abi_ty: ty,
        kind: AbiReturnKind::Plain,
    })
}

/// Recognizes the empty tuple used by status-only COM methods.
fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

/// Parsed generic components of a borrowed `ComRef` parameter.
struct ComRefType {
    /// Interface ABI type named by the wrapper.
    interface_ty: Type,
    /// Optional explicit threading marker.
    mode_ty: Option<Type>,
}

/// Parsed generic components of an owned `ComPtr` return.
struct ComPtrType {
    /// Interface ABI type named by the wrapper.
    interface_ty: Type,
    /// Optional explicit threading marker.
    mode_ty: Option<Type>,
}

/// Parsed element type of a bounded offset-buffer wrapper.
struct BufferOffsetType {
    /// Plain ABI element copied by the generated thunk.
    element_ty: Type,
}

/// Parses a call-scoped borrowed interface wrapper when `ty` names `ComRef`.
fn parse_com_ref_type(ty: &Type) -> Result<Option<ComRefType>> {
    let Some(args) = type_generic_args_named(ty, "ComRef") else {
        return Ok(None);
    };
    let generic_args = args.args.iter().collect::<Vec<_>>();
    if !(generic_args.len() == 2 || generic_args.len() == 3) {
        return Err(syn::Error::new_spanned(
            args,
            "`ComRef` parameters must be `ComRef<'_, InterfaceAbi>` or `ComRef<'_, InterfaceAbi, ThreadingMode>`",
        ));
    }
    require_call_scoped_lifetime(generic_args[0], "ComRef")?;
    let interface_ty = match generic_args[1] {
        syn::GenericArgument::Type(ty) => ty.clone(),
        arg => {
            return Err(syn::Error::new_spanned(
                arg,
                "`ComRef` interface parameter must be a type",
            ));
        }
    };
    let mode_ty = match generic_args.get(2) {
        Some(syn::GenericArgument::Type(ty)) => Some((*ty).clone()),
        Some(arg) => {
            return Err(syn::Error::new_spanned(
                arg,
                "`ComRef` threading mode parameter must be a type",
            ));
        }
        None => None,
    };

    Ok(Some(ComRefType {
        interface_ty,
        mode_ty,
    }))
}

/// Parses an immutable source-buffer parameter wrapper.
fn parse_copy_from_buffer_offset_type(ty: &Type) -> Result<Option<BufferOffsetType>> {
    parse_buffer_offset_type(
        ty,
        "NcCopyFromBufferOffset",
        "`NcCopyFromBufferOffset` parameters must be `NcCopyFromBufferOffset<'_, Element>`",
    )
}

/// Parses a mutable destination-buffer parameter wrapper.
fn parse_copy_to_buffer_offset_type(ty: &Type) -> Result<Option<BufferOffsetType>> {
    parse_buffer_offset_type(
        ty,
        "NcCopyToBufferOffset",
        "`NcCopyToBufferOffset` parameters must be `NcCopyToBufferOffset<'_, Element>`",
    )
}

/// Parses the common lifetime and element arguments of either buffer wrapper.
fn parse_buffer_offset_type(
    ty: &Type,
    expected_ident: &str,
    shape_error: &str,
) -> Result<Option<BufferOffsetType>> {
    let Some(args) = type_generic_args_named(ty, expected_ident) else {
        return Ok(None);
    };
    let generic_args = args.args.iter().collect::<Vec<_>>();
    if generic_args.len() != 2 {
        return Err(syn::Error::new_spanned(args, shape_error));
    }
    require_call_scoped_lifetime(generic_args[0], expected_ident)?;
    let element_ty = match generic_args[1] {
        syn::GenericArgument::Type(ty) => ty.clone(),
        arg => {
            return Err(syn::Error::new_spanned(
                arg,
                "buffer element parameter must be a type",
            ));
        }
    };

    Ok(Some(BufferOffsetType { element_ty }))
}

/// Requires `'_` so generated borrows cannot claim a longer source lifetime.
fn require_call_scoped_lifetime(arg: &syn::GenericArgument, wrapper: &str) -> Result<()> {
    match arg {
        syn::GenericArgument::Lifetime(lifetime) if lifetime.ident == "_" => Ok(()),
        _ => Err(syn::Error::new_spanned(
            arg,
            format!(
                "`{wrapper}` parameters must use `'_` as their call-scoped lifetime; named lifetimes, including `'static`, are not supported"
            ),
        )),
    }
}

/// Parses an owned interface wrapper when `ty` names `ComPtr`.
fn parse_com_ptr_type(ty: &Type) -> Result<Option<ComPtrType>> {
    let Some(args) = type_generic_args_named(ty, "ComPtr") else {
        return Ok(None);
    };
    let generic_args = args.args.iter().collect::<Vec<_>>();
    if !(generic_args.len() == 1 || generic_args.len() == 2) {
        return Err(syn::Error::new_spanned(
            args,
            "`ComPtr` parameters must be `ComPtr<InterfaceAbi>` or `ComPtr<InterfaceAbi, ThreadingMode>`",
        ));
    }
    let interface_ty = match generic_args[0] {
        syn::GenericArgument::Type(ty) => ty.clone(),
        arg => {
            return Err(syn::Error::new_spanned(
                arg,
                "`ComPtr` interface parameter must be a type",
            ));
        }
    };
    let mode_ty = match generic_args.get(1) {
        Some(syn::GenericArgument::Type(ty)) => Some((*ty).clone()),
        Some(arg) => {
            return Err(syn::Error::new_spanned(
                arg,
                "`ComPtr` threading mode parameter must be a type",
            ));
        }
        None => None,
    };

    Ok(Some(ComPtrType {
        interface_ty,
        mode_ty,
    }))
}

/// Returns angle-bracketed arguments when a type's final segment matches a wrapper.
fn type_generic_args_named<'a>(
    ty: &'a Type,
    expected_ident: &str,
) -> Option<&'a syn::AngleBracketedGenericArguments> {
    // Match the terminal segment so aliases such as `nc_com::ComPtr<T>` work
    // without coupling macro expansion to one dependency name or import style.
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != expected_ident {
        return None;
    }
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => Some(args),
        _ => None,
    }
}

/// Builds the mutable raw interface pointer used in generated C signatures.
fn raw_interface_pointer_ty(interface_ty: &Type) -> Type {
    syn::parse_quote! { *mut #interface_ty }
}

/// Retains only user rustdoc attributes for propagation to generated methods.
fn method_doc_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect()
}

/// Generates rustdoc describing one unsafe raw vtable slot and its contracts.
pub(crate) fn raw_vtable_method_docs(
    trait_ident: &Ident,
    method: &AbiMethod,
) -> Vec<proc_macro2::TokenStream> {
    let method_ident = &method.ident;
    let arg_docs = method.args.iter().flat_map(raw_vtable_arg_docs);
    let status_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful."]
        },
    };
    let safety_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "`this` must be a non-null valid pointer. Arguments lowered from `ComRef` must be valid borrowed interface pointers for the duration of the call."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "`this` and `out` must be non-null valid pointers. Arguments lowered from `ComRef` must be valid borrowed interface pointers for the duration of the call."]
        },
    };
    let return_docs = raw_vtable_return_docs(&method.ok);

    vec![quote! {
        #[doc = concat!("Raw COM ABI vtable slot for [`", stringify!(#trait_ident), "::", stringify!(#method_ident), "`].")]
        #[doc = ""]
        #status_doc
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #safety_doc
        #(#arg_docs)*
        #(#return_docs)*
    }]
}

/// Generates pointer, bounds, and ownership documentation for one raw argument.
fn raw_vtable_arg_docs(arg: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::Plain => {
            let abi_ty = &arg.abi_ty;
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed by value as `", stringify!(#abi_ty), "`.")]
            }]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is a borrowed `", stringify!(#interface_ty), "` interface pointer. The callee must call `AddRef` before retaining it after the call returns.")]
            }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` lowers to `offset: u64`, `len: u64`, and a readable `*const ", stringify!(#element_ty), "` buffer.")]
            }]
        }
        AbiArgKind::CopyToBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` lowers to `offset: u64`, `len: u64`, and a writable `*mut ", stringify!(#element_ty), "` buffer.")]
            }]
        }
    }
}

/// Generates output-slot documentation for one lowered return value.
fn raw_vtable_return_docs(ok: &AbiReturn) -> Vec<proc_macro2::TokenStream> {
    match &ok.kind {
        AbiReturnKind::Void => {
            vec![quote! {
                #[doc = "On success, no output value is written."]
            }]
        }
        AbiReturnKind::Plain => {
            let abi_ty = &ok.abi_ty;
            vec![quote! {
                #[doc = concat!("On success, `out` receives `", stringify!(#abi_ty), "`.")]
            }]
        }
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("On success, `out` receives an owned `", stringify!(#interface_ty), "` interface pointer transferred to the caller.")]
            }]
        }
    }
}

/// Generates rustdoc for a safe trait method or smart-pointer forwarding method.
pub(crate) fn safe_method_docs(
    trait_ident: &Ident,
    method: &AbiMethod,
) -> Vec<proc_macro2::TokenStream> {
    let method_ident = &method.ident;
    let arg_docs = method.args.iter().flat_map(safe_arg_docs);
    let wrapper_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful."]
        },
    };
    let return_docs = safe_return_docs(&method.ok);

    vec![quote! {
        #[doc = ""]
        #[doc = concat!("Calls [`", stringify!(#trait_ident), "::", stringify!(#method_ident), "`] through the generated COM ABI vtable.")]
        #[doc = ""]
        #wrapper_doc
        #[doc = "Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`."]
        #(#arg_docs)*
        #(#return_docs)*
    }]
}

/// Describes safe-wrapper ownership and bounds for one source-level argument.
fn safe_arg_docs(arg: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::Plain => {
            let rust_ty = &arg.rust_ty;
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed by value as `", stringify!(#rust_ty), "`.")]
            }]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` borrows a `", stringify!(#interface_ty), "` interface for the duration of the call. The callee must call `AddRef` before retaining it.")]
            }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed as a safe readable `", stringify!(#element_ty), "` buffer plus offset.")]
            }]
        }
        AbiArgKind::CopyToBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed as a safe writable `", stringify!(#element_ty), "` buffer plus offset.")]
            }]
        }
    }
}

/// Describes status-only, copied, or transferred successful return values.
fn safe_return_docs(ok: &AbiReturn) -> Vec<proc_macro2::TokenStream> {
    match &ok.kind {
        AbiReturnKind::Void => {
            vec![quote! {
                #[doc = "On success, returns `()`."]
            }]
        }
        AbiReturnKind::Plain => {
            let rust_ty = &ok.rust_ty;
            vec![quote! {
                #[doc = concat!("On success, returns `", stringify!(#rust_ty), "` from the ABI out slot.")]
            }]
        }
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let rust_ty = &ok.rust_ty;
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("On success, adopts the owned `", stringify!(#interface_ty), "` interface pointer returned through `out` as `", stringify!(#rust_ty), "`.")]
            }]
        }
    }
}

/// Recognizes `NcStatus` by the final segment of a possibly qualified path.
fn is_nc_status_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "NcStatus"),
        _ => false,
    }
}
