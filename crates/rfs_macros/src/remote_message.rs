//! This module derives a remote message enum for each remote method.
//! The enum contains 2 variants: the request and response.
//!
//! The request variant contains the function args, and the response contains the return value.

use proc_macro2::{extra::DelimSpan, Span};
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Field};

const VARIANT_REQUEST: &str = "Request";
const VARIANT_RESPONSE: &str = "Response";

/// Construct the enum
pub fn derive_enum(trait_name: syn::Ident, trait_method: syn::TraitItemFn) -> proc_macro2::TokenStream {
    let modified_method_ident = {
        let str = trait_method.sig.ident.to_string();
        let formatted_ident = camel_case_to_pascal_case(&str);

        syn::Ident::new(
            &format!("{}{}Message", trait_name, formatted_ident),
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

    // syn::DataEnum {
    //     enum_token: syn::token::Enum {
    //         span: trait_method.sig.ident.span(),
    //     },
    //     brace_token: Default::default(),
    //     variants: [request_variant, response_variant].into_iter().collect(),
    // }

    quote! {
        enum #modified_method_ident {
            #request_variant,
            #response_variant
        }
    }
}

/// Converts `camel_case` to `CamelCase`
fn camel_case_to_pascal_case(input: &str) -> String {
    input
        .split("_")
        .map(|item| {
            let mut chars = item.chars().collect::<Vec<_>>();

            match chars.first_mut() {
                Some(c) => *c = c.to_ascii_uppercase(),
                None => (),
            }

            chars.iter().collect::<String>()
        })
        .collect::<String>()
}

/// Convert a single pattern `name: bool` into it's equivalent struct field.
fn pat_to_struct_field(pat: &syn::PatType) -> syn::Field {
    let ident = if let syn::Pat::Ident(i) = *pat.pat.clone() {
        i.ident
    } else {
        unimplemented!("function signatures only contain identifiers")
    };

    syn::Field {
        attrs: Default::default(),
        vis: syn::Visibility::Inherited,
        mutability: syn::FieldMutability::None,
        ident: Some(ident),
        colon_token: Default::default(),
        ty: *pat.ty.clone(),
    }
}
