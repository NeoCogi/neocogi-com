//! Translates Cargo's normal artifact environment into transitive native linkage.
//!
//! The caller is the nightly-only native integration crate. This emits metadata,
//! performs no build, and never discovers directories or creates another graph.

use quote::quote;
use syn::LitStr;

/// Resolves one Cargo-owned artifact directory during target crate compilation.
/// Cargo fingerprints the normal artifact dependency that supplies this value.
pub(crate) fn expand(variable: LitStr) -> syn::Result<proc_macro2::TokenStream> {
    let directory = std::env::var(variable.value())
        .map_err(|error| syn::Error::new(variable.span(), format!("Cargo runtime artifact directory is unavailable: {error}")))?;
    if !std::path::Path::new(&directory).is_absolute() {
        return Err(syn::Error::new(variable.span(), "Cargo runtime artifact directory must be absolute"));
    }
    let unix_search = format!("-L{directory}");
    let msvc_search = format!("/LIBPATH:{directory}");
    // link-arg passes raw linker arguments, not cc-driver -Wl strings. Keep the
    // rpath option and its value on one block so rustc preserves their order.
    Ok(quote! {
        #[cfg(not(target_env = "msvc"))]
        #[link(kind = "link-arg", name = #unix_search)]
        unsafe extern "C" {}
        #[cfg(target_env = "msvc")]
        #[link(kind = "link-arg", name = #msvc_search)]
        unsafe extern "C" {}
        #[cfg(unix)]
        #[link(kind = "link-arg", name = "-rpath")]
        #[link(kind = "link-arg", name = #directory)]
        unsafe extern "C" {}
    })
}
