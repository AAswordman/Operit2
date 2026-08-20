use proc_macro::TokenStream;

/// Preserves one method-level Binding route declaration for proxy code generation.
#[proc_macro_attribute]
pub fn operit_core_route(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Preserves an internal method marker while excluding it from generated public Core APIs.
#[proc_macro_attribute]
pub fn operit_core_internal(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
