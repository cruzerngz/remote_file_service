#![allow(unused)]

use quote::quote;
use syn::{punctuated::Punctuated, ItemTrait};

mod remote_message;
mod remote_method_signature;

#[proc_macro_attribute]
pub fn remote_message_from_trait(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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

    let methods = items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(f) = item {
            Some(f)
        } else {
            None
        }

        // match item {
        //     syn::TraitItem::Const(_) => todo!(),
        //     syn::TraitItem::Fn(_) => todo!(),
        //     syn::TraitItem::Type(_) => todo!(),
        //     syn::TraitItem::Macro(_) => todo!(),
        //     syn::TraitItem::Verbatim(_) => todo!(),
        //     _ => todo!(),
        // }
    });

    let derived_enums = methods
        .map(|m| remote_message::derive_enum(ident.clone(), m.to_owned()))
        .collect::<proc_macro2::TokenStream>();

    derived_enums.into()
}
