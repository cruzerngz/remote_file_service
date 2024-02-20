//! This module contains a macro that derives a copy of a remote interface method.

use std::{num::Saturating, process::id};

use proc_macro2::Span;
// use proc_macro::Span;
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute, BareFnArg, Block,
    DeriveInput, FnArg, Ident, ItemTrait, Pat, Signature, Token, TraitItem, TraitItemFn,
};

use crate::{
    camel_case_to_pascal_case,
    remote_message::{VARIANT_REQUEST, VARIANT_RESPONSE},
};

const PAYLOAD_IDENT: &str = "payload";

/// Extend each method of a trait with a copy.
pub fn extend_trait(trait_def: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
    } = syn::parse_macro_input!(trait_def);

    // create the new trait items
    let new_trait_items = items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(f) = item {
                Some(f)
            } else {
                None
            }
        })
        .map(|trait_method| {
            let extended_fn = extend_method(ident.clone(), trait_method);

            [trait_method.to_owned(), extended_fn]
        })
        .flatten()
        .map(|func| TraitItem::Fn(func))
        .collect::<Vec<_>>();

    // ret updated trait
    ItemTrait {
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
        items: new_trait_items,
    }
    .to_token_stream()
    .into()
}

/// Extend a single trait method from the existing method
fn extend_method(trait_name: Ident, method: &TraitItemFn) -> TraitItemFn {
    // construct the enum name
    let enum_name: Ident = Ident::new(
        &camel_case_to_pascal_case(&format!("{}_{}", trait_name, method.sig.ident)),
        trait_name.span(),
    );

    let payload_ident = Ident::new(PAYLOAD_IDENT, Span::call_site());
    let fn_params: Punctuated<FnArg, Comma> = syn::parse_quote! {#payload_ident: #enum_name};

    // function arguments (the identifiers only)
    let fn_args = method
        .sig
        .inputs
        .iter()
        .map(|param| {
            let identifier = match param {
                FnArg::Receiver(_) => panic!("fn cannot be a receiver"),
                FnArg::Typed(arg) => {
                    if let Pat::Ident(i) = &*arg.pat {
                        i.ident.to_owned()
                    } else {
                        panic!("function params must be an identifier")
                    }
                }
            };

            identifier

            // Ident::new("asd", Span::call_site())
        })
        .collect::<Punctuated<_, Comma>>();

    let original_method_ident = method.sig.ident.clone();
    let req_variant = Ident::new(VARIANT_REQUEST, Span::call_site());
    let resp_variant = Ident::new(VARIANT_RESPONSE, Span::call_site());

    // contents of the function body
    let fn_body: Block = syn::parse2(quote! {{


        match #payload_ident {
            #enum_name::#req_variant {
                #fn_args
            } => {
                Self::#original_method_ident(
                    #fn_args
                ).await
            },
            #enum_name::#resp_variant(_) =>
            panic!("this method should only be called when the payload is a request."),
        }

    }})
    .expect("function body should be valid");

    let fn_name = Ident::new(&format!("{}_payload", method.sig.ident), method.span());

    let new_sig = Signature {
        ident: fn_name,
        inputs: fn_params,

        ..method.sig.to_owned()
    };

    let blank_attr: Attribute = syn::parse_quote! {
        #[doc = ""]
    };

    let comment_attr: Attribute = syn::parse_quote! {
        #[doc = concat!(
            "This method is derived from [`",
            stringify!(#trait_name),
            "::",
            stringify!(#original_method_ident),
            "`] and is implemented automatically."
        )]
    };

    let mut appended_attrs = method.attrs.to_owned();
    appended_attrs.push(blank_attr);
    appended_attrs.push(comment_attr);

    TraitItemFn {
        attrs: appended_attrs,
        sig: new_sig,
        default: Some(fn_body),

        ..method.to_owned()
    }
}
