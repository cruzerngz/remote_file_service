#![allow(unused)]

use quote::quote;
use syn::{punctuated::Punctuated, ItemTrait};

mod client;
mod remote_message;
pub(crate) mod remote_method_signature;

/// Generates the necessary code to implement a remote interface.
///
/// As a general rule, paramaters for the remote call can be of any type.
/// The return type of the call must be an owned type.
#[proc_macro_attribute]
pub fn remote_interface(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_cloned = proc_macro2::TokenStream::from(item.clone());

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
        .clone()
        .map(|m| {
            let (enum_ident, tokens) = remote_message::derive_enum(ident.clone(), m.to_owned());

            let remote_sig_derive =
                remote_method_signature::derive(enum_ident, &format!("{}::{}", ident, m.sig.ident));

            [tokens, remote_sig_derive]
        })
        .flatten()
        .collect::<proc_macro2::TokenStream>();

    // pass back the trait definition
    let trait_def = quote! {
        #[async_trait::async_trait]
        #item_cloned
    };

    let derived_client_impl =
        client::derive_client(ident.clone(), methods.map(|m| m.to_owned()).collect());

    [trait_def, derived_enums, derived_client_impl]
        .into_iter()
        .collect::<proc_macro2::TokenStream>()
        .into()
}
