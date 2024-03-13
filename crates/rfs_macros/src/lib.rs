#![allow(unused)]

use quote::quote;
use syn::{punctuated::Punctuated, ItemTrait};

mod client_builder;
mod extend_remote_callback;
mod extend_remote_interface;
mod remote_callback;
mod remote_message;
pub(crate) mod remote_method_signature;

/// Generates the necessary code to implement a remote interface.
///
/// As a general rule, parameters and return values can be of any type as long
/// as they satisfy these constraints:
/// - they are owned types (references are not allowed)
/// - they are concrete types (generics are not allowed)
/// - they implement serde's `Serialize` and `Deserialize`
///
/// ```ignore
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

    let trait_methods = items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(f) = item {
            Some(f)
        } else {
            None
        }
    });

    let (derived_enum_idents_sigs, derived_enums): (Vec<_>, Vec<_>) = trait_methods
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
        client_builder::derive_client(ident.clone(), trait_methods.map(|m| m.to_owned()).collect());

    [trait_def, derived_enums, derived_client_impl]
        .into_iter()
        .collect::<proc_macro2::TokenStream>()
        .into()
}

/// Create a remote callback.
///
/// NOT used at the moment
#[proc_macro_attribute]
pub fn remote_callback(
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

    let trait_methods = items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(f) = item {
            Some(f)
        } else {
            None
        }
    });

    let (derived_enum_idents_sigs, derived_enums): (Vec<_>, Vec<_>) = trait_methods
        .clone()
        .map(|method| {
            let (enum_ident, tokens) =
                remote_callback::derive_enum(ident.clone(), method.to_owned());

            let remote_sig_derive = remote_method_signature::derive(
                enum_ident.clone(),
                &format!("{}::{}", ident, method.sig.ident),
            );

            (
                (enum_ident, method.sig.to_owned()),
                [tokens, remote_sig_derive]
                    .into_iter()
                    .collect::<proc_macro2::TokenStream>(),
            )
        })
        .unzip();

    todo!()
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

// create a log::trace! macro by tagging the function name with the #[::function_name::named] attribute and then using
// the function_name! macro to get the function name.
fn create_trace_macro(ident: &syn::Ident) -> syn::ItemMacro {
    let function_name = syn::Ident::new("function_name", ident.span());
    let function_name_named = syn::Ident::new("named", ident.span());

    syn::parse_quote! {
        #[macro_export]
        macro_rules! trace {
            ($($arg:tt)*) => (log::trace!(concat!("[", #function_name!(), "] ", $($arg)*)))
        }
    }
}

#[proc_macro]
pub fn trace(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}
