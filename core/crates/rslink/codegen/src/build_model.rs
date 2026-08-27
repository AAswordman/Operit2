use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SourceRoot {
    pub src: PathBuf,
    pub crate_name: String,
}

impl SourceRoot {
    /// Creates a source root with the Rust crate name used in generated paths.
    pub fn new(src: PathBuf, crate_name: impl Into<String>) -> Self {
        Self {
            src,
            crate_name: crate_name.into(),
        }
    }

    /// Returns a borrowed source root for scanners that only need paths.
    pub fn as_path(&self) -> &Path {
        &self.src
    }
}

#[derive(Clone, Debug)]
pub struct ObjectSpec {
    pub object_id: u32,
    pub schema_key: String,
    pub dispatch_name: String,
    pub type_name: String,
    pub full_type: String,
    pub source_path: PathBuf,
    pub access: ObjectAccess,
    pub path_match: ObjectPathMatch,
}

/// Describes how concrete proxy paths map to one generated object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectPathMatch {
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectAccess {
    Application,
    ResolvedHolder {
        holder_field: String,
        resolver_method: String,
        proxy_aliases: Vec<(String, String)>,
    },
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
    CoreProxyConstruct,
    CoreNodeLocalRuntimeConstruct,
    StorePathsConstruct,
    ResultStorePathsConstruct,
    FactoryMethodConstruct {
        parent_schema_key: String,
        parent_full_type: String,
        parent_access: Box<ObjectAccess>,
        factory_method: String,
        factory_arg_types: Vec<String>,
        returns_result: bool,
        returns_arc_mutex: bool,
    },
}

impl ObjectAccess {
    pub fn is_constructible(&self) -> bool {
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
                | ObjectAccess::CoreProxyConstruct
                | ObjectAccess::CoreNodeLocalRuntimeConstruct
                | ObjectAccess::StorePathsConstruct
                | ObjectAccess::ResultStorePathsConstruct
                | ObjectAccess::FactoryMethodConstruct { .. }
        )
    }

    /// Returns whether this object can create child proxy objects through methods.
    pub fn supports_factory_methods(&self) -> bool {
        matches!(self, ObjectAccess::Application) || self.is_constructible()
    }
}

#[derive(Clone, Debug)]
pub struct PublicObjectType {
    pub type_name: String,
    pub full_type: String,
    pub source_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct TypeRegistry {
    pub aliases: HashMap<String, String>,
    pub trait_impls: HashMap<String, HashSet<String>>,
    pub stream_items: HashMap<String, String>,
}

impl TypeRegistry {
    pub fn resolve_alias(&self, ty: &str) -> String {
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

    pub fn stream_item(&self, ty: &str) -> Option<String> {
        let resolved = self.resolve_alias(ty);
        self.stream_items.get(&resolved).cloned()
    }
}

#[derive(Clone, Debug)]
pub struct SourceObject {
    pub object_id: u32,
    pub schema_key: String,
    pub dispatch_name: String,
    pub full_type: String,
    pub access: ObjectAccess,
    pub path_match: ObjectPathMatch,
    pub methods: Vec<SourceMethod>,
}

impl SourceObject {
    /// Returns whether generated call dispatch has at least one routable arm.
    pub fn has_call_dispatch(&self) -> bool {
        self.schema_key == "application"
            || self
                .methods
                .iter()
                .any(|method| method.call_protocol().is_some())
    }

    /// Returns whether generated sync call dispatch has direct non-async calls.
    pub fn has_sync_call_dispatch(&self) -> bool {
        self.methods
            .iter()
            .any(|method| !method.is_async && method.call_protocol().is_some())
    }

    /// Returns whether generated proxy calls need the typed value helper.
    pub fn has_proxy_value_call_methods(&self) -> bool {
        self.methods.iter().any(|method| {
            matches!(
                method.call_protocol(),
                Some(CallProtocol::Value(_) | CallProtocol::ResultValue { .. })
            )
        })
    }

    /// Returns whether generated proxy calls need the unit helper.
    pub fn has_proxy_unit_call_methods(&self) -> bool {
        self.methods.iter().any(|method| {
            matches!(
                method.call_protocol(),
                Some(CallProtocol::Unit | CallProtocol::ResultUnit { .. })
            )
        })
    }

    /// Returns whether generated proxy watches need the snapshot helper.
    pub fn has_proxy_snapshot_watch_methods(&self) -> bool {
        self.methods.iter().any(|method| {
            matches!(
                method.watch_protocol(),
                Some(WatchProtocol {
                    snapshot_type: Some(_),
                    item_type: _,
                    stream: WatchStreamProtocol::JsonFlow { .. }
                        | WatchStreamProtocol::JsonState { .. },
                })
            )
        })
    }
}

#[derive(Clone, Debug)]
pub struct SourceMethod {
    pub name: String,
    pub args: Vec<SourceArg>,
    pub rust_return_type: String,
    pub is_async: bool,
    pub cfg_attrs: Vec<String>,
    pub doc_lines: Vec<String>,
    pub protocol: MethodProtocol,
}

#[derive(Clone, Debug)]
pub struct SourceArg {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct SerializableType {
    pub full_type: String,
    pub supports_serialize: bool,
    pub supports_deserialize: bool,
    pub kind: SerializableTypeKind,
}

#[derive(Clone, Debug)]
pub enum SerializableTypeKind {
    Struct {
        fields: Vec<SerializableField>,
    },
    TaggedEnum {
        externally_tagged: bool,
        tag_name: Option<String>,
        content_name: Option<String>,
        variants: Vec<SerializableEnumVariant>,
    },
    Enum {
        variants: Vec<SerializableEnumVariant>,
        unit_only: bool,
    },
}

#[derive(Clone, Debug)]
pub struct SerializableField {
    pub name: String,
    pub json_name: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct SerializableEnumVariant {
    pub name: String,
    pub json_name: String,
    pub fields_are_unit: bool,
    pub fields_are_named: bool,
    pub fields: Vec<SerializableField>,
}

#[derive(Clone, Debug)]
pub struct ErrorTypeDefinition {
    pub full_type: String,
    pub variants: Vec<ErrorEnumVariant>,
}

#[derive(Clone, Debug)]
pub struct ErrorEnumVariant {
    pub name: String,
    pub fields_kind: ErrorFieldsKind,
    pub fields: Vec<ErrorField>,
}

#[derive(Clone, Debug)]
pub enum ErrorFieldsKind {
    Unit,
    Named,
    Unnamed,
}

#[derive(Clone, Debug)]
pub struct ErrorField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub enum MethodProtocol {
    Call(CallProtocol),
    Watch(WatchProtocol),
    ReverseStream(ReverseStreamProtocol),
    Factory(FactoryProtocol),
    Unsupported(String),
}

#[derive(Clone, Debug)]
pub enum CallProtocol {
    Unit,
    ResultUnit {
        error_type: String,
    },
    Value(String),
    ResultValue {
        value_type: String,
        error_type: String,
    },
}

#[derive(Clone, Debug)]
pub struct WatchProtocol {
    pub snapshot_type: Option<String>,
    pub item_type: String,
    pub stream: WatchStreamProtocol,
}

#[derive(Clone, Debug)]
pub struct ReverseStreamProtocol {
    pub argument_name: String,
    pub item_type: String,
}

#[derive(Clone, Debug)]
pub struct FactoryProtocol {
    pub target_schema_key: String,
}

#[derive(Clone, Debug)]
pub enum WatchStreamProtocol {
    JsonFlow { fallible: bool },
    JsonState { fallible: bool },
    JsonStream,
    StringStream,
}

impl SourceMethod {
    pub fn call_protocol(&self) -> Option<&CallProtocol> {
        match &self.protocol {
            MethodProtocol::Call(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn watch_protocol(&self) -> Option<&WatchProtocol> {
        match &self.protocol {
            MethodProtocol::Watch(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn factory_protocol(&self) -> Option<&FactoryProtocol> {
        match &self.protocol {
            MethodProtocol::Factory(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn reverse_stream_protocol(&self) -> Option<&ReverseStreamProtocol> {
        match &self.protocol {
            MethodProtocol::ReverseStream(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn unsupported_reason(&self) -> Option<&str> {
        match &self.protocol {
            MethodProtocol::Unsupported(reason) => Some(reason),
            _ => None,
        }
    }
}
