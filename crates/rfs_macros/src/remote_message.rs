//! This module derives a remote message enum for each remote method.
//! The enum contains 2 variants: the request and response.
//!
//! The request variant contains the function args, and the response contains the return value.

use proc_macro2::{extra::DelimSpan, Span};
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Field};

use crate::{camel_case_to_pascal_case, pat_to_struct_field};

pub(crate) const VARIANT_REQUEST: &str = "Request";
pub(crate) const VARIANT_RESPONSE: &str = "Response";

/// Construct the enum.
///
/// Returns the enum ident and the enum as a tokenstream.
pub fn derive_enum(
    trait_name: syn::Ident,
    trait_method: syn::TraitItemFn,
) -> (syn::Ident, proc_macro2::TokenStream) {
    let modified_method_ident = {
        let str = trait_method.sig.ident.to_string();
        let formatted_ident = camel_case_to_pascal_case(&str);

        syn::Ident::new(
            &format!("{}{}", trait_name, formatted_ident),
            trait_method.sig.ident.span(),
        )
    };

    let inputs = trait_method.sig.inputs;
    let ret_val = trait_method.sig.output;

    let request_variant_fields = inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Receiver(rv) => unimplemented!("trait method must not have a receiver"),

            syn::FnArg::Typed(typ) => pat_to_struct_field(typ),
        })
        .collect::<Punctuated<Field, Comma>>();

    let resp_variant_type = match ret_val {
        // unit type
        syn::ReturnType::Default => syn::Type::Tuple(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: Default::default(),
        }),

        syn::ReturnType::Type(_, ty) => *ty,
    };

    let request_variant = syn::Variant {
        attrs: Default::default(),
        ident: syn::Ident::new(VARIANT_REQUEST, Span::call_site()),
        fields: syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: request_variant_fields,
        }),
        discriminant: None,
    };

    let response_variant = syn::Variant {
        attrs: Default::default(),
        ident: syn::Ident::new(VARIANT_RESPONSE, Span::call_site()),
        fields: syn::Fields::Unnamed(syn::FieldsUnnamed {
            paren_token: Default::default(),
            unnamed: [resp_variant_type]
                .iter()
                .map(|ty| Field {
                    attrs: Default::default(),
                    vis: syn::Visibility::Inherited,
                    mutability: syn::FieldMutability::None,
                    ident: None,
                    colon_token: Default::default(),
                    ty: ty.clone(),
                })
                .collect(),
        }),
        discriminant: None,
    };

    let cloned_ident = modified_method_ident.clone();

    (
        cloned_ident,
        quote! {
            #[doc = "Method call payload"]
            #[doc = ""]
            #[doc = concat!("This enum is automatically generated from [`", stringify!(#trait_name), "`]")]
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub enum #modified_method_ident {
                #request_variant,
                #response_variant
            }
        },
    )
}
