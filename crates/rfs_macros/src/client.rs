//! Logic for deriving a client data structure.
//!

use std::{cell::OnceCell, sync::Arc};

use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{Block, FnArg, ImplItemFn, Pat, Signature, TraitItemFn};

const ADDITIONAL_ARG: OnceCell<FnArg> = OnceCell::new();

/// The identifier for the context manager
const CTX_MGR_IDENT: &str = "ctx";

fn initialize_additional_arg() -> FnArg {
    let x: FnArg = syn::parse2(quote! {ctx: ContextManager}).unwrap();

    FnArg::Typed(syn::PatType {
        attrs: vec![],
        pat: Box::new(Pat::Ident(syn::PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: Ident::new("string", Span::call_site()),
            subpat: None,
        })),
        colon_token: Default::default(),
        ty: Box::new(syn::Type::Path(syn::TypePath {
            qself: None,
            path: syn::Path {
                leading_colon: todo!(),
                segments: todo!(),
            },
        })),
    })
}

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
        syn::parse2(quote! {ctx: rfs_core::middleware::ContextManager}).unwrap();

    // same thing for this one
    #[allow(non_snake_case)]
    let FUNC_BODY: Block = syn::parse2(quote! {{
            let x = false;
            todo!()
    }})
    .unwrap();

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
            signature.inputs.insert(0, NEW_FUNC_ARG.clone());

            let new_method = ImplItemFn {
                attrs: method.attrs,
                vis: syn::Visibility::Public(syn::token::Pub {
                    span: Span::call_site(),
                }),
                defaultness: None,
                sig: signature,
                block: FUNC_BODY.clone(),
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
