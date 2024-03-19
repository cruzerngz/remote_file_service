//! Logic for deriving a client data structure.
//!

use std::{cell::OnceCell, fmt::format, sync::Arc};

use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Block, Field, FieldValue, FnArg,
    Generics, ImplItemFn, Pat, ReturnType, Signature, TraitItemFn,
};

use crate::{
    camel_case_to_pascal_case,
    remote_message::{VARIANT_REQUEST, VARIANT_RESPONSE},
};

/// The identifier for the context manager
const CTX_MGR_IDENT: &str = "ctx";

/// From the trait name, derive a new client struct and implement
/// the same methods as the trait, but with an additional parameter:
/// the context manager.
///
/// The context manager is the middleware that handles communication with the
/// remote.
pub fn derive_client(
    trait_name: Ident,
    trait_methods: Vec<TraitItemFn>,
) -> proc_macro2::TokenStream {
    // I can't seem to define this as a global without going through
    // ten thousand steps, so I'm just going to define it here.
    #[allow(non_snake_case)]
    let NEW_FUNC_ARG: FnArg =
        syn::parse2(quote! {ctx: &mut rfs_core::middleware::ContextManager}).unwrap();

    // struct definition
    let struct_name = Ident::new(&format!("{}Client", &trait_name), trait_name.span());
    let struct_def = quote! {
        #[doc = "Client for method invocations."]
        #[doc = ""]
        #[doc = concat!("This struct is automatically generated from [`", stringify!(#trait_name), "`]")]
        #[derive(Debug)]
        pub struct #struct_name;
    };

    let impl_methods = trait_methods
        .into_iter()
        .map(|method| {
            let mut signature = method.sig;

            let request_builder = func_call_to_enum_request(
                signature.inputs.clone(),
                Ident::new(
                    &camel_case_to_pascal_case(&format!("{}_{}", trait_name, signature.ident)),
                    signature.ident.span(),
                ),
            );

            signature.inputs.insert(0, NEW_FUNC_ARG.clone());
            signature.output = wrap_in_result(
                signature.output,
                syn::parse2(quote! {rfs_core::middleware::InvokeError}).unwrap(),
            );

            // Generic should match `NEW_FUNC_ARG`
            // check if trait ident matches the definition
            signature.generics =
                syn::parse_quote! {
                // <T: rfs_core::middleware::TransmissionProtocol + core::marker::Send + core::marker::Sync>
            };

            let new_method = ImplItemFn {
                attrs: method.attrs,
                vis: syn::Visibility::Public(syn::token::Pub {
                    span: Span::call_site(),
                }),
                defaultness: None,
                sig: signature.to_owned(),
                block: syn::parse2(quote! {{

                    #request_builder

                }})
                .expect("block parsing should not fail"),
            };

            new_method.to_token_stream()
        })
        .collect::<proc_macro2::TokenStream>();

    let impl_block = quote! {
        impl #struct_name {
            #impl_methods
        }
    };

    // TraitItemFn;
    [struct_def, impl_block].into_iter().collect()
}

/// Generates the code block to transform a set of parameters to an enum request.
///
/// The enum is assumesd to contain the named variant [`VARIANT_REQUEST`].
///
/// The enum request variant is also assumed to match the order, types and number
/// of arguments exactly.
fn func_call_to_enum_request(
    fn_params: Punctuated<FnArg, Comma>,
    enum_ident: Ident,
) -> proc_macro2::TokenStream {
    // we use the field init shorthand
    let mut enum_params = fn_params
        .into_iter()
        .map(|fn_p| {
            let typed = match fn_p {
                FnArg::Receiver(r) => panic!("args should not contain self"),
                FnArg::Typed(t) => t,
            };

            let param_ident = if let Pat::Ident(i) = &*typed.pat {
                &i.ident
            } else {
                panic!("function arg should be an identifier")
            };

            param_ident.to_owned()
        })
        .collect::<Punctuated<Ident, Comma>>();

    let req_variant = Ident::new(VARIANT_REQUEST, Span::call_site());
    let resp_variant = Ident::new(VARIANT_RESPONSE, Span::call_site());

    // TODO: remove the unwraps and return a result instead
    quote! {
        let request = #enum_ident::#req_variant {
            #enum_params
        };

        let response = ctx.invoke(request).await?;

        match response {
            #enum_ident::#req_variant{..} => unimplemented!("this branch is never taken"),
            #enum_ident::#resp_variant(value) => return Ok(value)
        }
    }
}

/// Transform the given return type as a result with an error.
fn wrap_in_result(mut ret: ReturnType, err_type: syn::Path) -> ReturnType {
    match ret {
        ReturnType::Default => syn::parse2::<ReturnType>(quote! {
            -> Result<(), #err_type>
        })
        .unwrap(),
        ReturnType::Type(t, ty) => syn::parse2::<ReturnType>(quote! {
            -> Result<#ty, #err_type>
        })
        .unwrap(),
    }
}
