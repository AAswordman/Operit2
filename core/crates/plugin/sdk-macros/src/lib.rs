//! Compiler-recognized metadata consumed from Rust source by the SDK declaration generator.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

/// Marks declaration scope, public name, category, and object-wide field mutability.
///
/// The declaration generator reads this attribute from the original Rust AST.
/// Rust retains the original types and fields, with `#[js(readonly)]` field metadata
/// removed before normal compilation. No JavaScript runtime code is generated here.
#[proc_macro_attribute]
pub fn js_declaration(arguments: TokenStream, input: TokenStream) -> TokenStream {
    let _arguments = parse_macro_input!(arguments with syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated);
    let mut item = parse_macro_input!(input as Item);
    match &mut item {
        Item::Struct(item) => {
            for field in &mut item.fields {
                for attribute in field.attrs.iter().filter(|attribute| attribute.path().is_ident("js")) {
                    let mut readonly = false;
                    if let Err(error) = attribute.parse_nested_meta(|meta| {
                        if !meta.path.is_ident("readonly") {
                            return Err(meta.error("unknown JavaScript field option"));
                        }
                        if readonly { return Err(meta.error("duplicate readonly option")); }
                        readonly = true;
                        Ok(())
                    }) {
                        return error.into_compile_error().into();
                    }
                }
                field.attrs.retain(|attribute| !attribute.path().is_ident("js"));
            }
        }
        Item::Enum(_) | Item::Trait(_) | Item::Type(_) => {}
        _ => return syn::Error::new_spanned(item, "js_declaration requires a struct, enum, trait, or type alias")
            .into_compile_error().into(),
    }
    quote!(#item).into()
}
