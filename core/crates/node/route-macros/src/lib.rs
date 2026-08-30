use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input, Error, FnArg, GenericArgument, Ident, ImplItemFn, Pat, PathArguments,
    ReturnType, Token, Type,
};

struct CoreRouteArguments {
    binding: Ident,
}

impl Parse for CoreRouteArguments {
    /// Parses the binding identifier owned by one routed method annotation.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        if name != "binding" {
            return Err(Error::new(name.span(), "expected `binding = argument`"));
        }
        input.parse::<Token![=]>()?;
        let binding = input.parse::<Ident>()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("only one `binding = argument` entry is supported"));
        }
        Ok(Self { binding })
    }
}

/// Wraps one routed method and preserves its local implementation under a generated name.
#[proc_macro_attribute]
pub fn operit_core_route(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = parse_macro_input!(attribute as CoreRouteArguments);
    let function = parse_macro_input!(item as ImplItemFn);
    match expand_core_route(arguments, function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Generates the public routed entry point and its protocol-safe local implementation.
fn expand_core_route(
    arguments: CoreRouteArguments,
    mut function: ImplItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    let original_name = function.sig.ident.clone();
    let local_name = format_ident!("__operit_core_local_{}", original_name);
    let binding_argument = arguments.binding;
    let argument_names = function_argument_names(&function)?;
    if !argument_names.iter().any(|name| name == &binding_argument) {
        return Err(Error::new(
            binding_argument.span(),
            format!(
                "routed method `{original_name}` does not declare binding argument `{binding_argument}`"
            ),
        ));
    }

    let local_signature = {
        let mut signature = function.sig.clone();
        signature.ident = local_name.clone();
        signature
    };
    let local_body = function.block.clone();
    let preserved_attrs = function
        .attrs
        .iter()
        .filter(|attribute| !attribute.path().is_ident("operit_core_route"))
        .cloned()
        .collect::<Vec<_>>();
    let visibility = function.vis.clone();
    let receiver = function.sig.receiver().is_some();
    let call_arguments = argument_names.iter().map(|name| quote!(#name));
    let local_call = if receiver {
        quote!(self.#local_name(#(#call_arguments),*))
    } else {
        quote!(Self::#local_name(#(#call_arguments),*))
    };
    let call_expression = if function.sig.asyncness.is_some() {
        quote!(#local_call.await)
    } else {
        local_call
    };
    let wrapper_signature = function.sig.clone();
    let wrapper_attrs = preserved_attrs.clone();
    let binding_name = binding_argument.to_string();
    let route_metadata_name = format_ident!("__operit_core_binding_{}", original_name);
    let call_helper_name = format_ident!("__operit_core_route_call_{}", original_name);
    let watch_snapshot_helper_name =
        format_ident!("__operit_core_route_watch_snapshot_{}", original_name);
    let watch_helper_name = format_ident!("__operit_core_route_watch_{}", original_name);
    let route_arg_entries = argument_names
        .iter()
        .map(|name| {
            quote! {
                operit_rslink_runtime::core_arg_entry(stringify!(#name), &#name)
                    .expect("annotated route argument must be serializable")
            }
        })
        .collect::<Vec<_>>();
    let route_args_value = quote! {
        operit_rslink_runtime::core_route_args(vec![#(#route_arg_entries),*])
    };
    let route_call_decode = route_call_decode(&function.sig.output);
    let route_wrapper_body = if return_type_is_state_flow(&function.sig.output) {
        let remote_flow = quote! {
            match operit_rslink_runtime::core_route_state_flow(
                __core_route_runtime.clone(),
                stringify!(#original_name),
                __core_route_args_value,
            )
                .await
            {
                Ok(__core_route_flow) => return __core_route_flow,
                Err(__core_route_error) => panic!(
                    "annotated route watch failed: {:?}",
                    __core_route_error
                ),
            }
        };
        let local_source_flow = quote! {
            match operit_rslink_runtime::core_route_state_flow_with_local_source(
                __core_route_runtime.clone(),
                stringify!(#original_name),
                __core_route_args_value,
                #call_expression,
            )
                .await
            {
                Ok(__core_route_flow) => return __core_route_flow,
                Err(__core_route_error) => panic!(
                    "annotated route watch with local source failed: {:?}",
                    __core_route_error
                ),
            }
        };
        if function.sig.asyncness.is_some() {
            quote! {
                if let Some(__core_route_runtime) = operit_link::coreRouteRuntime() {
                    let __core_route_args_value = #route_args_value;
                    if __core_route_runtime
                        .shouldRouteWatch(stringify!(#original_name), &__core_route_args_value)
                        .expect("annotated route resolution failed") {
                        #remote_flow
                    }
                    if __core_route_runtime
                        .shouldUseLocalWatchSource(stringify!(#original_name), &__core_route_args_value)
                        .expect("annotated route local source resolution failed") {
                        #local_source_flow
                    }
                }
                #call_expression
            }
        } else {
            quote! {
                #call_expression
            }
        }
    } else if function.sig.asyncness.is_some() {
        quote! {
            if let Some(__core_route_runtime) = operit_link::coreRouteRuntime() {
                let __core_route_args_value = #route_args_value;
                if __core_route_runtime
                    .shouldRoute(stringify!(#original_name), &__core_route_args_value)
                    .expect("annotated route resolution failed") {
                    let __core_route_response = operit_rslink_runtime::core_route_call_response(
                            __core_route_runtime.as_ref(),
                            stringify!(#original_name),
                            __core_route_args_value,
                        )
                        .await;
                    return #route_call_decode;
                }
            }
            #call_expression
        }
    } else {
        quote! {
            #call_expression
        }
    };

    let route_helpers = render_route_helpers(
        &function,
        &local_name,
        &call_helper_name,
        &watch_snapshot_helper_name,
        &watch_helper_name,
    );

    function.sig = local_signature.clone();
    function.block = local_body;
    function.attrs = preserved_attrs;
    function.vis = visibility.clone();

    Ok(quote! {
        #(#wrapper_attrs)*
        #visibility
        #wrapper_signature {
            #route_wrapper_body
        }

        #[doc(hidden)]
        #function

        #[doc(hidden)]
        #visibility fn #route_metadata_name() -> &'static str {
            #binding_name
        }

        #route_helpers
    })
}

/// Generates protocol-facing helpers inside the annotated implementation.
fn render_route_helpers(
    function: &ImplItemFn,
    local_name: &Ident,
    call_helper_name: &Ident,
    watch_snapshot_helper_name: &Ident,
    watch_helper_name: &Ident,
) -> proc_macro2::TokenStream {
    let argument_specs = function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => match argument.pat.as_ref() {
                Pat::Ident(pattern) => Some((pattern.ident.clone(), argument.ty.clone())),
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let decode_statements = argument_specs
        .iter()
        .map(|(name, ty)| {
            let wire_type = owned_decode_type(ty);
            let call_expression = if matches!(ty.as_ref(), Type::Reference(_)) {
                quote! { &#name }
            } else {
                quote! { #name }
            };
            quote! {
                let #name: #wire_type = operit_rslink_runtime::decode_core_arg(
                    &mut __core_args,
                    stringify!(#name),
                )?;
                let _ = &#call_expression;
            }
        })
        .collect::<Vec<_>>();
    let call_arguments = argument_specs
        .iter()
        .map(|(name, ty)| {
            if matches!(ty.as_ref(), Type::Reference(_)) {
                quote! { &#name }
            } else {
                quote! { #name }
            }
        })
        .collect::<Vec<_>>();
    let call_expression = if function.sig.asyncness.is_some() {
        quote! { self.#local_name(#(#call_arguments),*).await }
    } else {
        quote! { self.#local_name(#(#call_arguments),*) }
    };
    let return_encoding = render_return_encoding(&function.sig.output);
    let is_state_flow = return_type_is_state_flow(&function.sig.output);
    let call_helper_body = quote! {
        #[doc(hidden)]
        pub async fn #call_helper_name(
            &mut self,
            request: operit_link::CoreCallRequest,
        ) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
            let mut __core_args = operit_rslink_runtime::object_args(request.args)?;
            #(#decode_statements)*
            let __core_result = #call_expression;
            #return_encoding
        }
    };

    let call_helper = if is_state_flow {
        quote! {}
    } else {
        call_helper_body
    };
    if !is_state_flow {
        return quote! { #call_helper };
    }
    let snapshot_expression = if function.sig.asyncness.is_some() {
        quote! { self.#local_name(#(#call_arguments),*).await.value() }
    } else {
        quote! { self.#local_name(#(#call_arguments),*).value() }
    };
    let watch_expression = if function.sig.asyncness.is_some() {
        quote! { self.#local_name(#(#call_arguments),*).await }
    } else {
        quote! { self.#local_name(#(#call_arguments),*) }
    };
    let watch_snapshot_helper = quote! {
        #[doc(hidden)]
        pub async fn #watch_snapshot_helper_name(
            &mut self,
            request: &operit_link::CoreWatchRequest,
        ) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
            let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;
            #(#decode_statements)*
            operit_rslink_runtime::to_core_value(#snapshot_expression)
        }
    };
    let watch_helper = quote! {
        #[doc(hidden)]
        pub async fn #watch_helper_name(
            &mut self,
            request: operit_link::CoreWatchRequest,
            attachmentAdopter: std::sync::Arc<
                dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync,
            >,
        ) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {
            let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;
            #(#decode_statements)*
            let state_flow = #watch_expression;
            operit_rslink_runtime::core_state_flow_event_stream(
                state_flow,
                request,
                attachmentAdopter,
            )
        }
    };
    quote! { #call_helper #watch_snapshot_helper #watch_helper }
}

/// Returns an owned type suitable for decoding one wire argument.
fn owned_decode_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Reference(reference) => quote! { #reference.elem },
        _ => quote! { #ty },
    }
}

/// Renders Result-aware Link encoding for one annotated method return type.
fn render_return_encoding(output: &ReturnType) -> proc_macro2::TokenStream {
    let ReturnType::Type(_, ty) = output else {
        return quote! { Ok(operit_link::CoreValue::Null) };
    };
    if is_unit_type(ty) {
        return quote! { Ok(operit_link::CoreValue::Null) };
    }
    if let Some((ok_type, _error_type)) = result_arguments(ty) {
        if is_unit_type(ok_type) {
            return quote! {
                match __core_result {
                    Ok(()) => Ok(operit_link::CoreValue::Null),
                    Err(error) => Err(operit_link::CoreLinkError::internal(error.to_string())),
                }
            };
        }
        return quote! {
            match __core_result {
                Ok(value) => operit_rslink_runtime::to_core_value(value),
                Err(error) => Err(operit_link::CoreLinkError::internal(error.to_string())),
            }
        };
    }
    quote! {
        operit_rslink_runtime::to_core_value(__core_result)
    }
}

/// Returns the two generic arguments of a Result type.
fn result_arguments(ty: &Type) -> Option<(&Type, &Type)> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    Some((types.next()?, types.next()?))
}

/// Returns whether a type is the unit type.
fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

/// Returns whether a type is the standard String type.
fn is_string_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "String")
}

/// Returns whether a method returns a StateFlow.
fn return_type_is_state_flow(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(path) = ty.as_ref() else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "StateFlow")
}

/// Decodes one routed call response into the annotated method's declared return type.
fn route_call_decode(output: &ReturnType) -> proc_macro2::TokenStream {
    let ReturnType::Type(_, ty) = output else {
        return quote! { () };
    };
    if is_unit_type(ty) {
        return quote! {
            operit_rslink_runtime::decode_core_call_unit_response(__core_route_response)
                .expect("annotated route call failed");
            ()
        };
    }
    if let Some((ok_type, error_type)) = result_arguments(ty) {
        if is_unit_type(ok_type) {
            if is_string_type(error_type) {
                return quote! {
                    operit_rslink_runtime::decode_core_call_unit_response(__core_route_response)
                        .map_err(|error| error.to_string())
                };
            }
            return quote! {
                {
                    operit_rslink_runtime::decode_core_call_unit_response(__core_route_response)
                        .expect("annotated route call failed");
                    Ok(())
                }
            };
        }
        if is_string_type(error_type) {
            return quote! {
                operit_rslink_runtime::decode_core_call_response::<#ok_type>(__core_route_response)
                    .map_err(|error| error.to_string())
            };
        }
        return quote! {
            Ok(
                operit_rslink_runtime::decode_core_call_response::<#ok_type>(__core_route_response)
                    .expect("annotated route response failed to decode")
            )
        };
    }
    quote! {
        operit_rslink_runtime::decode_core_call_response(__core_route_response)
        .expect("annotated route response failed to decode")
    }
}

/// Extracts simple identifier arguments used by the generated local wrapper call.
fn function_argument_names(function: &ImplItemFn) -> syn::Result<Vec<Ident>> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(match argument.pat.as_ref() {
                Pat::Ident(pattern) => Ok(pattern.ident.clone()),
                pattern => Err(Error::new_spanned(
                    pattern,
                    "routed methods require identifier argument patterns",
                )),
            }),
        })
        .collect()
}

/// Preserves an internal method marker while excluding it from generated public Core APIs.
#[proc_macro_attribute]
pub fn operit_core_internal(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the lifecycle callback invoked immediately before a Core route change.
#[proc_macro_attribute]
pub fn before_change_route(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the lifecycle callback invoked after a Core route reaches its target.
#[proc_macro_attribute]
pub fn after_change_route(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
