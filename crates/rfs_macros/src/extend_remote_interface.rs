//! This module contains a macro that derives a copy of a remote interface method.

use syn::{ItemTrait, TraitItemFn};

/// Extend each method of a trait with a copy.
pub fn extend_trait(trait_def: proc_macro::TokenStream) -> proc_macro2::TokenStream {
    // let ItemTrait {
    //     attrs,
    //     vis,
    //     unsafety,
    //     auto_token,
    //     restriction,
    //     trait_token,
    //     ident,
    //     generics,
    //     colon_token,
    //     supertraits,
    //     brace_token,
    //     items,
    // } = syn::parse_macro_input!(trait_def);

    todo!()
}

/// Extend a single trait method from the existing method
fn extend_method(method: &TraitItemFn) -> TraitItemFn {
    todo!()
}
