use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    Fields, ImplItem, Item, ItemEnum, ItemImpl, ItemStruct, Meta, Token, Type, UseTree, Visibility,
};

mod build_model;
mod build_type_resolver;
mod build_utils;

pub use build_model::*;
pub use build_type_resolver::{
    borrowed_slice_inner, collect_error_type_definitions, collect_serializable_type_definitions,
    collect_type_registry, core_stream_inner, flow_inner, generic_args, is_supported_arg_type,
    is_supported_return_type, normalize_type, result_unit_error_type, result_value_parts,
    single_generic_arg, split_top_level_args, state_flow_inner, TypeResolver,
};
pub use build_utils::{
    dispatch_name_from_schema_key, full_type_for_source_with_crate, identifier_words, lower_first,
    module_path_for_source_with_crate, parent_module_path,
};
