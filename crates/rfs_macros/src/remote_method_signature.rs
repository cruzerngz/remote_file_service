//! Logic for deriving the trait `RemoteMethodSignature`.
//!

use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, ItemTrait};

const REMOTE_METHOD_SIG_TRAIT: &str = "RemoteMethodSignature";
const REMOTE_METHOD_SIG_TRAIT_METHOD: &str = "remote_method_signature";

/// Implement the trait `RemoteMethodSignature` with the given method signature.
pub fn derive(identifier: syn::Ident, signature: &str) -> proc_macro2::TokenStream {
    let trait_name = syn::Ident::new(REMOTE_METHOD_SIG_TRAIT, Span::call_site());
    let trait_method = syn::Ident::new(REMOTE_METHOD_SIG_TRAIT_METHOD, Span::call_site());

    quote! {
        impl #trait_name for #identifier {
            fn #trait_method() -> &'static [u8] {
                #signature.as_bytes()
            }
        }

    }
    .into()
}
