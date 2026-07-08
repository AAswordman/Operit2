use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct SourceRoot {
    pub(crate) src: PathBuf,
    pub(crate) crate_name: String,
}

impl SourceRoot {
    /// Creates a source root with the Rust crate name used in generated paths.
    pub(crate) fn new(src: PathBuf, crate_name: impl Into<String>) -> Self {
        Self {
            src,
            crate_name: crate_name.into(),
        }
    }

    /// Returns a borrowed source root for scanners that only need paths.
    pub(crate) fn as_path(&self) -> &Path {
        &self.src
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectSpec {
    pub(crate) schema_key: String,
    pub(crate) dispatch_name: String,
    pub(crate) type_name: String,
    pub(crate) full_type: String,
    pub(crate) source_path: PathBuf,
    pub(crate) access: ObjectAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ObjectAccess {
    Application,
    ChatRuntimeMain,
    DefaultConstruct,
    GetInstanceConstruct,
    ResultGetInstanceConstruct,
    NewConstruct,
    StringNewConstruct,
    ContextGetInstanceConstruct,
    ContextRefGetInstanceConstruct,
    ResultContextGetInstanceConstruct,
    ResultContextRefGetInstanceConstruct,
    ContextGetInstanceArcMutexConstruct,
    ContextRefGetInstanceArcMutexConstruct,
    StorePathsConstruct,
    ResultStorePathsConstruct,
    FactoryMethodConstruct {
        parent_schema_key: String,
        parent_full_type: String,
        parent_access: Box<ObjectAccess>,
        factory_method: String,
        factory_arg_types: Vec<String>,
    },
}

impl ObjectAccess {
    pub(crate) fn is_constructible(&self) -> bool {
        matches!(
            self,
            ObjectAccess::DefaultConstruct
                | ObjectAccess::GetInstanceConstruct
                | ObjectAccess::ResultGetInstanceConstruct
                | ObjectAccess::NewConstruct
                | ObjectAccess::StringNewConstruct
                | ObjectAccess::ContextGetInstanceConstruct
                | ObjectAccess::ContextRefGetInstanceConstruct
                | ObjectAccess::ResultContextGetInstanceConstruct
                | ObjectAccess::ResultContextRefGetInstanceConstruct
                | ObjectAccess::ContextGetInstanceArcMutexConstruct
                | ObjectAccess::ContextRefGetInstanceArcMutexConstruct
                | ObjectAccess::StorePathsConstruct
                | ObjectAccess::ResultStorePathsConstruct
                | ObjectAccess::FactoryMethodConstruct { .. }
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PublicObjectType {
    pub(crate) type_name: String,
    pub(crate) full_type: String,
    pub(crate) source_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TypeRegistry {
    pub(crate) aliases: HashMap<String, String>,
    pub(crate) trait_impls: HashMap<String, HashSet<String>>,
    pub(crate) stream_items: HashMap<String, String>,
}

impl TypeRegistry {
    pub(crate) fn resolve_alias(&self, ty: &str) -> String {
        let mut current = ty.to_string();
        let mut visited = HashSet::new();
        while visited.insert(current.clone()) {
            let Some(next) = self.aliases.get(&current) else {
                break;
            };
            current = next.clone();
        }
        current
    }

    pub(crate) fn implements(&self, ty: &str, trait_name: &str) -> bool {
        let resolved = self.resolve_alias(ty);
        self.trait_impls
            .get(&resolved)
            .map(|traits| traits.contains(trait_name))
            .unwrap_or(false)
    }

    pub(crate) fn stream_item(&self, ty: &str) -> Option<String> {
        let resolved = self.resolve_alias(ty);
        self.stream_items.get(&resolved).cloned()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceObject {
    pub(crate) schema_key: String,
    pub(crate) dispatch_name: String,
    pub(crate) full_type: String,
    pub(crate) access: ObjectAccess,
    pub(crate) methods: Vec<SourceMethod>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceMethod {
    pub(crate) name: String,
    pub(crate) args: Vec<SourceArg>,
    pub(crate) rust_return_type: String,
    pub(crate) is_async: bool,
    pub(crate) cfg_attrs: Vec<String>,
    pub(crate) protocol: MethodProtocol,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceArg {
    pub(crate) name: String,
    pub(crate) ty: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SerializableType {
    pub(crate) full_type: String,
    pub(crate) supports_serialize: bool,
    pub(crate) supports_deserialize: bool,
    pub(crate) kind: SerializableTypeKind,
}

#[derive(Clone, Debug)]
pub(crate) enum SerializableTypeKind {
    Struct {
        fields: Vec<SerializableField>,
    },
    TaggedEnum {
        tag_name: String,
        content_name: Option<String>,
        variants: Vec<SerializableEnumVariant>,
    },
    Enum {
        variants: Vec<SerializableEnumVariant>,
        unit_only: bool,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct SerializableField {
    pub(crate) name: String,
    pub(crate) json_name: String,
    pub(crate) ty: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SerializableEnumVariant {
    pub(crate) name: String,
    pub(crate) json_name: String,
    pub(crate) fields_are_unnamed: bool,
    pub(crate) fields: Vec<SerializableField>,
}

#[derive(Clone, Debug)]
pub(crate) struct ErrorTypeDefinition {
    pub(crate) full_type: String,
    pub(crate) variants: Vec<ErrorEnumVariant>,
}

#[derive(Clone, Debug)]
pub(crate) struct ErrorEnumVariant {
    pub(crate) name: String,
    pub(crate) fields_kind: ErrorFieldsKind,
    pub(crate) fields: Vec<ErrorField>,
}

#[derive(Clone, Debug)]
pub(crate) enum ErrorFieldsKind {
    Unit,
    Named,
    Unnamed,
}

#[derive(Clone, Debug)]
pub(crate) struct ErrorField {
    pub(crate) name: String,
    pub(crate) ty: String,
}

#[derive(Clone, Debug)]
pub(crate) enum MethodProtocol {
    Call(CallProtocol),
    Watch(WatchProtocol),
    Factory(FactoryProtocol),
    Unsupported(String),
}

#[derive(Clone, Debug)]
pub(crate) enum CallProtocol {
    Unit,
    ResultUnit { error_type: String },
    Value(String),
    ResultValue {
        value_type: String,
        error_type: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct WatchProtocol {
    pub(crate) snapshot_type: Option<String>,
    pub(crate) stream: WatchStreamProtocol,
}

#[derive(Clone, Debug)]
pub(crate) struct FactoryProtocol {
    pub(crate) target_schema_key: String,
}

#[derive(Clone, Debug)]
pub(crate) enum WatchStreamProtocol {
    JsonFlow { fallible: bool },
    JsonState { fallible: bool },
    JsonStream,
    StringStream,
    TextEvent { optional: bool },
}

impl SourceMethod {
    pub(crate) fn call_protocol(&self) -> Option<&CallProtocol> {
        match &self.protocol {
            MethodProtocol::Call(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub(crate) fn watch_protocol(&self) -> Option<&WatchProtocol> {
        match &self.protocol {
            MethodProtocol::Watch(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub(crate) fn factory_protocol(&self) -> Option<&FactoryProtocol> {
        match &self.protocol {
            MethodProtocol::Factory(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub(crate) fn unsupported_reason(&self) -> Option<&str> {
        match &self.protocol {
            MethodProtocol::Unsupported(reason) => Some(reason),
            _ => None,
        }
    }
}
