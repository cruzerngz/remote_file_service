//! Module for deriving a remote callback enum type.

use proc_macro2::Span;
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Field};

use crate::{camel_case_to_pascal_case, pat_to_struct_field};

pub(crate) const VARIANT_REGISTER: &str = "Register";
pub(crate) const VARIANT_REGISTER_ACK: &str = "Acknowledge";
pub(crate) const VARIANT_CALLBACK: &str = "Callback";

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

    let reg_variant_fields = inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Receiver(rv) => unimplemented!("trait method must not have a receiver"),

            syn::FnArg::Typed(typ) => pat_to_struct_field(typ),
        })
        .collect::<Punctuated<Field, Comma>>();

    let callback_variant_type = match ret_val {
        // unit type
        syn::ReturnType::Default => syn::Type::Tuple(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: Default::default(),
        }),

        syn::ReturnType::Type(_, ty) => *ty,
    };

    let register_variant = syn::Variant {
        attrs: Default::default(),
        ident: syn::Ident::new(VARIANT_REGISTER, Span::call_site()),
        fields: syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: reg_variant_fields,
        }),
        discriminant: None,
    };

    let callback_variant = syn::Variant {
        attrs: Default::default(),
        ident: syn::Ident::new(VARIANT_CALLBACK, Span::call_site()),
        fields: syn::Fields::Unnamed(syn::FieldsUnnamed {
            paren_token: Default::default(),
            unnamed: [callback_variant_type]
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
            #[doc = "Callback data payload"]
            #[doc = ""]
            #[doc = concat!("This enum is automatically generated from [`", stringify!(#trait_name), "`]")]
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub enum #modified_method_ident {
                #register_variant,
                #VARIANT_REGISTER_ACK,
                #callback_variant,
            }
        },
    )
}
