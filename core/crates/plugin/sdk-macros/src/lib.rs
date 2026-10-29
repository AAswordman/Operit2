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
                for attribute in field
                    .attrs
                    .iter()
                    .filter(|attribute| attribute.path().is_ident("js"))
                {
                    let mut readonly = false;
                    if let Err(error) = attribute.parse_nested_meta(|meta| {
                        if !meta.path.is_ident("readonly") {
                            return Err(meta.error("unknown JavaScript field option"));
                        }
                        if readonly {
                            return Err(meta.error("duplicate readonly option"));
                        }
                        readonly = true;
                        Ok(())
                    }) {
                        return error.into_compile_error().into();
                    }
                }
                field
                    .attrs
                    .retain(|attribute| !attribute.path().is_ident("js"));
            }
        }
        Item::Enum(_) | Item::Trait(_) | Item::Type(_) => {}
        _ => {
            return syn::Error::new_spanned(
                item,
                "js_declaration requires a struct, enum, trait, or type alias",
            )
            .into_compile_error()
            .into()
        }
    }
    quote!(#item).into()
}

/// Keeps display metadata beside each built-in declaration and emits its typed lookup.
#[proc_macro_attribute]
pub fn tool_display(_arguments: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as syn::ItemStruct);
    let mut arms = Vec::new();
    for field in &mut item.fields {
        let mut action = None;
        let mut icon = None;
        let mut target = None;
        let mut result = None;
        for attribute in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("display"))
        {
            if let Err(error) = attribute.parse_nested_meta(|meta| {
                let value: syn::LitStr = meta.value()?.parse()?;
                if meta.path.is_ident("action") {
                    action = Some(value);
                } else if meta.path.is_ident("icon") {
                    icon = Some(value);
                } else if meta.path.is_ident("target") {
                    target = Some(value);
                } else if meta.path.is_ident("result") {
                    result = Some(value);
                } else {
                    return Err(meta.error("unknown tool display field"));
                }
                Ok(())
            }) {
                return error.into_compile_error().into();
            }
        }
        field.attrs.retain(|attr| !attr.path().is_ident("display"));
        if let Some(action) = action {
            let name = field
                .ident
                .as_ref()
                .expect("tool declarations require named fields")
                .to_string();
            let icon = icon.unwrap_or_else(|| syn::LitStr::new("tool", action.span()));
            let target = match target {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            let renderer = match result.as_ref().map(syn::LitStr::value).as_deref() {
                None | Some("text") => quote!(Text),
                Some("directory") => quote!(Directory),
                Some("fileDiff") => quote!(FileDiff),
                _ => {
                    return syn::Error::new_spanned(result.unwrap(), "unknown result renderer")
                        .into_compile_error()
                        .into()
                }
            };
            arms.push(quote!(#name => Some(super::tool_display::ToolDisplayDeclaration::builtin(#action, #icon, #target, super::tool_display::ToolResultRenderer::#renderer)),));
        }
    }
    quote! {
        #item
        impl BuiltinToolName {
            /// Reads display semantics from this tool's declaration, without name heuristics.
            pub fn display(self) -> Option<super::tool_display::ToolDisplayDeclaration> {
                match self.as_str() { #(#arms)* _ => None }
            }
        }
    }
    .into()
}
