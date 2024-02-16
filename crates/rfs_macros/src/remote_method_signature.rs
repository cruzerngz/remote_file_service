//! Logic for deriving the trait `RemoteMethodSignature`.
//!

use quote::quote;
use syn::{DeriveInput, ItemTrait};

/// The input tokenstream should contain a trait definition.
///
/// The identifier should be some derived data structure from another macro.
pub fn derive(identifier: syn::Ident, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ItemTrait {
        attrs,
        vis,
        unsafety,
        auto_token,
        restriction,
        trait_token,
        ident,
        generics,
        colon_token,
        supertraits,
        brace_token,
        items,
    } = syn::parse_macro_input!(item);

    let trait_name = ident;

    // ItemTrait;

    quote! {}.into()
}
