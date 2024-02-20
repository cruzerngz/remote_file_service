#![allow(unused)]

use quote::quote;
use syn::{punctuated::Punctuated, ItemTrait};

mod client;
mod extend_remote_interface;
mod remote_message;
pub(crate) mod remote_method_signature;

/// Generates the necessary code to implement a remote interface.
///
/// As a general rule, parameters and return values can be of any type as long as
/// they are owned types.
///
/// ```no_run
/// /// This trait defines a remote interface.
/// ///
/// /// In the current implementation, traits do not supported provided methods
/// /// (default methods). All methods defined here must be implemented
/// /// by the remote.
/// #[remote_interface]
/// pub trait SomeMethods {
///     /// Methods must be declared as async, and must not contain
///     /// receivers (&self, &mut self).
///     ///
///     /// A mutable receiver will be added after processing by the macro.
///     async fn do_something(left: usize, right: usize) -> usize;
/// }
/// ```
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
    });

    let (derived_enum_idents_sigs, derived_enums): (Vec<_>, Vec<_>) = methods
        .clone()
        .map(|m| {
            let (enum_ident, tokens) = remote_message::derive_enum(ident.clone(), m.to_owned());

            let remote_sig_derive = remote_method_signature::derive(
                enum_ident.clone(),
                &format!("{}::{}", ident, m.sig.ident),
            );

            (
                (enum_ident, m.sig.to_owned()),
                [tokens, remote_sig_derive]
                    .into_iter()
                    .collect::<proc_macro2::TokenStream>(),
            )
        })
        .unzip();

    let derived_enums = derived_enums
        .into_iter()
        .collect::<proc_macro2::TokenStream>();

    // pass back the new trait definition
    let new_trait_def: proc_macro2::TokenStream =
        extend_remote_interface::extend_trait(item_cloned.into()).into();
    let trait_def = quote! {
        #[async_trait::async_trait]
        #new_trait_def
    };

    // generate client struct
    let derived_client_impl =
        client::derive_client(ident.clone(), methods.map(|m| m.to_owned()).collect());

    [trait_def, derived_enums, derived_client_impl]
        .into_iter()
        .collect::<proc_macro2::TokenStream>()
        .into()
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
