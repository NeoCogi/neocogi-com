//! Ordinary host entry expansion with explicit implementation ownership.
//!
//! The macro wires one selected host initializer and supplied access. It owns
//! no executor, registry lifecycle counter or universal implementation detector.

use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, Path, Result, Token};

use crate::codegen::util::{PrivateNames, runtime_crate_path};

/// Optional explicit paths for renamed facade and host implementation crates.
#[derive(Default)]
pub struct MainArgs {
    /// Canonical facade path unless overridden with `neocogi_com = path`.
    facade: Option<Path>,
    /// Ordinary host implementation path unless overridden with `nc_com = path`.
    implementation: Option<Path>,
}

impl Parse for MainArgs {
    /// Accepts only the two dependency paths, rejecting duplicate configuration.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut args = Self::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let slot = if key == "neocogi_com" {
                &mut args.facade
            } else if key == "nc_com" {
                &mut args.implementation
            } else {
                return Err(syn::Error::new(key.span(), "expected neocogi_com = path or nc_com = path"));
            };
            if slot.is_some() {
                return Err(syn::Error::new(key.span(), "duplicate com_main dependency path"));
            }
            input.parse::<Token![=]>()?;
            *slot = Some(input.parse()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}

/// Generates a no-argument entry that initializes the selected host exactly once
/// per invocation and calls its authored body with admitted access.
///
/// The authored body remains an ordinary nested function, retaining its return,
/// `?` and recursion semantics. Attributes stay on the public entry and inherited
/// lint scopes cover the body; cfg removes the entire generated implementation.
pub fn expand_com_main(args: MainArgs, input: ItemFn) -> Result<proc_macro2::TokenStream> {
    let signature = &input.sig;
    if signature.ident.unraw() != "main"
        || signature.inputs.len() != 1
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.constness.is_some()
        || matches!(signature.inputs.first(), Some(syn::FnArg::Receiver(_)))
    {
        return Err(syn::Error::new_spanned(
            signature,
            "com_main requires a safe, synchronous, non-generic Rust fn main(runtime: Runtime)",
        ));
    }
    let nc = runtime_crate_path(args.facade.as_ref());
    let implementation = args.implementation.unwrap_or_else(|| syn::parse_quote!(::nc_com));
    let mut names = PrivateNames::new(quote! { #input #nc #implementation });
    let host = names.binding("__nc_host_api");
    let runtime = names.binding("__nc_runtime");
    let imports = names.binding_imports();
    let attrs = &input.attrs;
    let vis = &input.vis;
    let mut entry = input.sig.clone();
    entry.inputs.clear();
    let mut body = input.clone();
    body.attrs.clear();
    body.vis = syn::Visibility::Inherited;
    let authored = &signature.ident;
    Ok(quote! {
        #(#attrs)*
        #vis #entry {
            #imports
            #body
            let #host = #implementation::initialize_process();
            // SAFETY: the explicit host implementation guarantees the complete
            // immutable process-lived service graph and callable code.
            let #runtime = unsafe { #nc::Runtime::__from_host_api(#host) }
                .expect("incompatible neocogi-com host API");
            #authored(#runtime)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{MainArgs, expand_com_main};

    /// Invalid entry shapes fail at the authored signature before emission.
    #[test]
    fn rejects_unsupported_entry_shapes() {
        for source in [
            "fn other(runtime: Runtime) {}",
            "fn main() {}",
            "fn main(a: Runtime, b: Runtime) {}",
            "unsafe fn main(runtime: Runtime) {}",
            "async fn main(runtime: Runtime) {}",
            "extern \"C\" fn main(runtime: Runtime) {}",
            "const fn main(runtime: Runtime) {}",
            "fn main<T>(runtime: Runtime) {}",
            "fn main(runtime: Runtime) where Runtime: Copy {}",
        ] {
            assert!(expand_com_main(MainArgs::default(), syn::parse_str(source).unwrap()).is_err(), "{source}");
        }
    }

    /// Path configuration is explicit and cannot silently select two owners.
    #[test]
    fn dependency_path_admission_is_unambiguous() {
        assert!(syn::parse_str::<MainArgs>("neocogi_com = client, nc_com = owner,").is_ok());
        for source in ["runtime = owner", "nc_com = a, nc_com = b", "neocogi_com = a, neocogi_com = b"] {
            assert!(syn::parse_str::<MainArgs>(source).is_err());
        }
    }
}
