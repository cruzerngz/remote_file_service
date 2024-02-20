//! This module derives the `RemoteCall` trait.
//! NOTUSED

use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, token::Comma, BareFnArg, Ident, Pat, Path, Signature, TypeBareFn,
};

use crate::remote_message::{VARIANT_REQUEST, VARIANT_RESPONSE};

pub fn derive_remote_call(payload_type: Ident, method_sig: Signature) -> proc_macro2::TokenStream {
    let param_idents = method_sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_) => unimplemented!("receivers are not supported"),
            syn::FnArg::Typed(t) => {
                if let Pat::Ident(i) = &*t.pat {
                    i.ident.to_owned()
                } else {
                    unimplemented!("function paramters can only be identifiers")
                }
            }
        })
        .collect::<Punctuated<_, Comma>>();

    let fn_pointer = wrap_fn_in_async(method_sig);

    let req_variant = Ident::new(VARIANT_REQUEST, Span::call_site());
    let resp_variant = Ident::new(VARIANT_RESPONSE, Span::call_site());

    quote! {
        #[async_trait::async_trait]
        impl rfs_core::RemoteCall for #payload_type {
            type Function = std::pin::Pin<Box<dyn #fn_pointer>>;

            async fn call(self, func: Self::Function) -> Self {

                let res = if let #payload_type::#req_variant {
                    #param_idents
                } = self {
                    func(
                        #param_idents
                    ).await
                } else {
                    panic!("this method can only be called on requests")
                };

                #payload_type::#resp_variant(res)
            }
        }
    }
}

/// Wraps a given function signature in async syntax vomit,
/// and returns a function pointer type.
fn wrap_fn_in_async(sig: Signature) -> syn::TypeTraitObject {
    let param_types = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_) => unimplemented!("receivers are not supported"),
            syn::FnArg::Typed(t) => BareFnArg {
                attrs: Default::default(),
                name: None,
                ty: (*t.ty).to_owned(),
            },
        })
        .collect::<Punctuated<BareFnArg, Comma>>();

    let ret_type = match sig.output {
        syn::ReturnType::Default => quote! {()},
        syn::ReturnType::Type(_, t) => t.to_token_stream(),
    };

    syn::parse2(quote! {
        Fn(#param_types) ->
            Box<dyn std::future::Future<Output = #ret_type > + Send>
    })
    .unwrap()
}
