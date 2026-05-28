use super::*;

pub(crate) fn render_generated(objects: &[SourceObject], schema_json: &str) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_core_proxy_schema() -> serde_json::Value {\n");
    output.push_str("    serde_json::from_str(r#\"");
    output.push_str(&schema_json);
    output.push_str("\"#).expect(\"generated core proxy schema must be valid JSON\")\n");
    output.push_str("}\n\n");
    for object in objects {
        output.push_str(&render_object_call_dispatch(object));
        output.push('\n');
        output.push_str(&render_object_watch_snapshot_dispatch(object));
        output.push('\n');
        output.push_str(&render_object_watch_dispatch(object));
        output.push('\n');
    }
    output.push_str(&render_core_proxy_dispatch(objects));
    output.push('\n');
    output.push_str(&render_generated_proxy(objects));
    output
}

pub(crate) fn render_schema(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    format!(
        "{{\"objects\":{},\"types\":{}}}",
        render_schema_objects(objects),
        render_schema_types(serializable_types)
    )
}

fn render_schema_objects(objects: &[SourceObject]) -> String {
    let entries = objects
        .iter()
        .map(|object| {
            format!(
                "{}:{}",
                json_string(&object.schema_key),
                render_schema_methods(&object.methods)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_schema_methods(methods: &[SourceMethod]) -> String {
    let entries = methods
        .iter()
        .map(|method| {
            let args = method
                .args
                .iter()
                .map(|arg| {
                    format!(
                        "{{\"name\":{},\"type\":{}}}",
                        json_string(&arg.name),
                        json_string(&arg.ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"name\":{},\"args\":[{}],\"async\":{},\"callable\":{},\"watchable\":{},\"returnType\":{},\"protocol\":{},\"unsupportedReason\":{}}}",
                json_string(&method.name),
                args,
                method.is_async,
                method.call_protocol().is_some(),
                method.watch_protocol().is_some(),
                json_string(&method.rust_return_type),
                render_schema_protocol(&method.protocol),
                option_json_string(method.unsupported_reason())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_schema_types(serializable_types: &HashMap<String, SerializableType>) -> String {
    let mut types = serializable_types.values().collect::<Vec<_>>();
    types.sort_by(|left, right| left.full_type.cmp(&right.full_type));
    let entries = types
        .iter()
        .map(|ty| match &ty.kind {
            SerializableTypeKind::Struct { fields } => {
                let fields_json = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{{\"name\":{},\"type\":{}}}",
                            json_string(&field.name),
                            json_string(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}:{{\"kind\":\"struct\",\"fields\":[{}]}}",
                    json_string(&ty.full_type),
                    fields_json
                )
            }
            SerializableTypeKind::Enum { variants } => {
                let variants_json = variants
                    .iter()
                    .map(|variant| json_string(variant))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}:{{\"kind\":\"enum\",\"variants\":[{}]}}",
                    json_string(&ty.full_type),
                    variants_json
                )
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_schema_protocol(protocol: &MethodProtocol) -> String {
    match protocol {
        MethodProtocol::Call(_) => {
            "{\"mode\":\"Call\",\"payload\":\"Json\",\"initial\":\"None\"}".to_string()
        }
        MethodProtocol::Watch(watch) => {
            let payload = match watch.stream {
                WatchStreamProtocol::JsonFlow { .. } | WatchStreamProtocol::JsonState { .. } => {
                    "Json"
                }
                WatchStreamProtocol::TextEvent { .. } => "TextStreamEvent",
            };
            let initial = if watch.snapshot_type.is_some() {
                "Snapshot"
            } else {
                "None"
            };
            format!("{{\"mode\":\"Watch\",\"payload\":\"{payload}\",\"initial\":\"{initial}\"}}")
        }
        MethodProtocol::Unsupported(_) => "null".to_string(),
    }
}

fn render_object_call_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_call(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args)?;\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.call_protocol().is_some())
    {
        output.push_str(&render_call_arm(method));
    }
    if object.schema_key == "application" {
        output.push_str("        \"coreProxySchema\" => Ok(generated_core_proxy_schema()),\n");
    }
    output
        .push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_object_watch_snapshot_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch_snapshot(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object.methods.iter().filter(|method| {
        method
            .watch_protocol()
            .and_then(|watch| watch.snapshot_type.as_ref())
            .is_some()
    }) {
        output.push_str(&render_watch_snapshot_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_object_watch_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch(object: &mut {}, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.watch_protocol().is_some())
    {
        output.push_str(&render_watch_stream_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_core_proxy_dispatch(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_call(proxy: &mut LocalCoreProxy, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {\n");
    output.push_str("    if request.targetPath.key() == \"application\" && request.methodName == \"runCoreCommand\" {\n");
    output.push_str("        let mut __core_args = object_args(request.args)?;\n");
    output.push_str("        let args: Vec<String> = decode_core_arg(&mut __core_args, \"args\")?;\n");
    output.push_str("        let output = tokio::task::block_in_place(|| operit_command_core::run_core_command(&mut proxy.application, &args)).map_err(|error| operit_link::CoreLinkError::internal(error))?;\n");
    output.push_str("        return to_core_value(output);\n");
    output.push_str("    }\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.targetPath.key() == {:?} {{\n        return generated_dispatch_{}_call(&mut proxy.application, request).await;\n    }}\n",
            application.schema_key, application.dispatch_name
        ));
    }
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_call(core, request).await;\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::Call,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct
    }) {
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            format!(
                "            generated_dispatch_{}_call(&mut object, request).await\n",
                object.dispatch_name
            )
        ));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),\n",
    );
    output.push_str("    }\n}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch_snapshot(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let propertyName = request.propertyName.clone();\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        let value = generated_dispatch_{}_watch_snapshot(core, &request)?;\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    let propertyName = request.propertyName.clone();\n");
    output.push_str("    let value = match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch_snapshot(&mut proxy.application, &request)?,\n",
            application.schema_key, application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::WatchSnapshot,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct
    }) {
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            format!(
                "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
                object.dispatch_name
            )
        ));
    }
    output.push_str("        _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    };\n");
    output.push_str("    Ok(operit_link::CoreEvent { requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value })\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_watch(core, request);\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch(&mut proxy.application, request),\n",
            application.schema_key, application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::Watch,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct
    }) {
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            format!(
                "            generated_dispatch_{}_watch(&mut object, request)\n",
                object.dispatch_name
            )
        ));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n",
    );
    output.push_str("    }\n}\n");
    output
}

#[derive(Clone, Copy)]
enum DispatchMode {
    Call,
    WatchSnapshot,
    Watch,
}

fn render_string_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    let base_segments = object.schema_key.split('.').collect::<Vec<_>>();
    let len = base_segments.len();
    let segment_checks = base_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "request.targetPath.segments.get({index}).map(String::as_str) == Some({segment:?})"
            )
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let dispatch = match mode {
        DispatchMode::Call => format!(
            "            return generated_dispatch_{}_call(&mut object, request).await;\n",
            object.dispatch_name
        ),
        DispatchMode::WatchSnapshot => format!(
            "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
            object.dispatch_name
        ),
        DispatchMode::Watch => format!(
            "            return generated_dispatch_{}_watch(&mut object, request);\n",
            object.dispatch_name
        ),
    };
    format!(
        "        _ if request.targetPath.segments.len() == {} && {} => {{\n{}{}        }}\n",
        len + 1,
        segment_checks,
        render_object_constructor(object),
        dispatch
    )
}

fn render_object_constructor(object: &SourceObject) -> String {
    match object.access {
        ObjectAccess::DefaultConstruct => {
            format!(
                "            let mut object = {}::default();\n",
                object.full_type
            )
        }
        ObjectAccess::GetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance();\n",
                object.full_type
            )
        }
        ObjectAccess::ResultGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::NewConstruct => {
            format!(
                "            let mut object = {}::new();\n",
                object.full_type
            )
        }
        ObjectAccess::StringNewConstruct => {
            let segment_index = object.schema_key.split('.').count();
            format!(
                "            let __core_instance_id = request.targetPath.segments.get({segment_index}).cloned().ok_or_else(|| operit_link::CoreLinkError::internal(\"missing object instance id\"))?;\n            let mut object = {}::new(__core_instance_id);\n",
                object.full_type
            )
        }
        ObjectAccess::ContextGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(proxy.application.applicationContext.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::ContextRefGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(&proxy.application.applicationContext);\n",
                object.full_type
            )
        }
        ObjectAccess::StorePathsConstruct => {
            format!(
                "            let mut object = {}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default());\n",
                object.full_type
            )
        }
        ObjectAccess::ResultStorePathsConstruct => {
            format!(
                "            let mut object = {}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::Application | ObjectAccess::ChatRuntimeMain => String::new(),
    }
}

fn render_generated_proxy(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str("pub struct GeneratedCoreProxy<C> {\n");
    output.push_str("    client: C,\n");
    output.push_str("}\n\n");
    output.push_str("impl<C: operit_link::CoreLinkClient> GeneratedCoreProxy<C> {\n");
    output.push_str("    pub fn new(client: C) -> Self {\n");
    output.push_str("        Self { client }\n");
    output.push_str("    }\n\n");
    output.push_str("    pub fn intoInner(self) -> C {\n");
    output.push_str("        self.client\n");
    output.push_str("    }\n\n");
    output.push_str("    pub fn clientMut(&mut self) -> &mut C {\n");
    output.push_str("        &mut self.client\n");
    output.push_str("    }\n\n");
    output.push_str("    pub async fn runCoreCommand(&mut self, args: &[String]) -> Result<operit_command_core::CoreCommandOutput, operit_link::CoreLinkError> {\n");
    output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), operit_link::CoreObjectPath::parse(\"application\"), \"runCoreCommand\", serde_json::json!({ \"args\": args }))).await;\n");
    output.push_str("        let value = response.result?;\n");
    output.push_str("        serde_json::from_value(value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
    output.push_str("    }\n\n");
    for object in objects {
        let proxy_type = proxy_object_type_name(object);
        if object.access == ObjectAccess::StringNewConstruct {
            output.push_str(&format!(
                "    pub fn {}(&mut self, instanceId: &str) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            let segments = object
                .schema_key
                .split('.')
                .map(|segment| format!("{segment:?}.to_string()"))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str("        let mut segments = vec![");
            output.push_str(&segments);
            output.push_str("];\n");
            output.push_str("        segments.push(instanceId.to_string());\n");
            output.push_str(&format!(
                "        {}::new(&mut self.client, operit_link::CoreObjectPath {{ segments }})\n",
                proxy_type
            ));
        } else {
            output.push_str(&format!(
                "    pub fn {}(&mut self) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            output.push_str(&format!(
                "        {}::new(&mut self.client, operit_link::CoreObjectPath::parse({:?}))\n",
                proxy_type, object.schema_key
            ));
        }
        output.push_str("    }\n\n");
    }
    output.push_str("}\n\n");

    for object in objects {
        let proxy_type = proxy_object_type_name(object);
        output.push_str(&format!("pub struct {}<'a, C> {{\n", proxy_type));
        output.push_str("    client: &'a mut C,\n");
        output.push_str("    target_path: operit_link::CoreObjectPath,\n");
        output.push_str("}\n\n");
        output.push_str(&format!(
            "impl<'a, C: operit_link::CoreLinkClient> {}<'a, C> {{\n",
            proxy_type
        ));
        output.push_str(
            "    fn new(client: &'a mut C, target_path: operit_link::CoreObjectPath) -> Self {\n",
        );
        output.push_str("        Self { client, target_path }\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn callGenerated<T: serde::de::DeserializeOwned>(&mut self, methodName: &str, args: serde_json::Value) -> Result<T, operit_link::CoreLinkError> {\n");
        output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), self.target_path.clone(), methodName, args)).await;\n");
        output.push_str("        let value = response.result?;\n");
        output.push_str("        serde_json::from_value(value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn callGeneratedUnit(&mut self, methodName: &str, args: serde_json::Value) -> Result<(), operit_link::CoreLinkError> {\n");
        output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), self.target_path.clone(), methodName, args)).await;\n");
        output.push_str("        response.result.map(|_| ())\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn watchGenerated<T: serde::de::DeserializeOwned>(&mut self, propertyName: &str, args: serde_json::Value) -> Result<T, operit_link::CoreLinkError> {\n");
        output.push_str("        let event = self.client.watchSnapshot(operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), propertyName, args)).await?;\n");
        output.push_str("        serde_json::from_value(event.value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
        output.push_str("    }\n\n");
        for method in object
            .methods
            .iter()
            .filter(|method| method.call_protocol().is_some())
        {
            output.push_str(&render_proxy_call_method(method));
        }
        for method in object
            .methods
            .iter()
            .filter(|method| method.watch_protocol().is_some())
        {
            output.push_str(&render_proxy_watch_method(object, method));
        }
        output.push_str(&render_proxy_watch_all_method(object));
        output.push_str("}\n\n");
    }
    output
}

fn proxy_object_type_name(object: &SourceObject) -> String {
    let mut out = String::from("GeneratedCoreProxy");
    for part in object.dispatch_name.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn render_proxy_call_method(method: &SourceMethod) -> String {
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    match method.call_protocol() {
        Some(CallProtocol::Unit | CallProtocol::ResultUnit) => format!(
            "    pub async fn {}(&mut self{}) -> Result<(), operit_link::CoreLinkError> {{\n        self.callGeneratedUnit({:?}, {}).await\n    }}\n\n",
            method.name, params, method.name, args_json
        ),
        Some(CallProtocol::Value(value) | CallProtocol::ResultValue(value)) => format!(
            "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.callGenerated({:?}, {}).await\n    }}\n\n",
            method.name, params, value, method.name, args_json
        ),
        None => String::new(),
    }
}

fn render_proxy_watch_method(object: &SourceObject, method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match &watch.stream {
        WatchStreamProtocol::TextEvent { .. } => {
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            format!(
                "    pub async fn {}(&mut self{}) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n        self.client.watch(operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), {:?}, {})).await\n    }}\n\n",
                method.name, params, method.name, args_json
            )
        }
        WatchStreamProtocol::JsonFlow { .. } | WatchStreamProtocol::JsonState { .. } => {
            let Some(value) = watch.snapshot_type.as_ref() else {
                return String::new();
            };
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            let mut output = format!(
                "    pub async fn {}Snapshot(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.watchGenerated({:?}, {}).await\n    }}\n\n",
                method.name, params, value, method.name, args_json
            );
            let Some(alias) = method.name.strip_suffix("Flow") else {
                return output;
            };
            if alias.is_empty() || object.methods.iter().any(|existing| existing.name == alias) {
                return output;
            }
            output.push_str(&format!(
                "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.watchGenerated({:?}, {}).await\n    }}\n\n",
                alias, params, value, method.name, args_json
            ));
            output
        }
    }
}

fn render_proxy_watch_all_method(object: &SourceObject) -> String {
    let watchable = object
        .methods
        .iter()
        .filter(|method| method.args.is_empty())
        .filter(|method| {
            method
                .watch_protocol()
                .and_then(|watch| watch.snapshot_type.as_ref())
                .is_some()
        })
        .map(|method| json_string(&method.name))
        .collect::<Vec<_>>();
    if watchable.is_empty() {
        return "    pub async fn watchAllGeneratedStateFlows(&mut self, _sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {\n        Ok(())\n    }\n\n".to_string();
    }
    format!(
        "    pub async fn watchAllGeneratedStateFlows(&mut self, sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {{\n        for propertyName in [{}] {{\n            let request = operit_link::CoreWatchRequest::new(generated_proxy_request_id(), {:?}, propertyName, serde_json::json!({{}}));\n            let mut stream = self.client.watch(request).await?;\n            let sender = sender.clone();\n            tokio::spawn(async move {{\n                while let Some(event) = stream.recv().await {{\n                    let _ = sender.send(event);\n                }}\n            }});\n        }}\n        Ok(())\n    }}\n\n",
        watchable.join(", "),
        object.schema_key
    )
}

fn render_proxy_params(method: &SourceMethod) -> String {
    if method.args.is_empty() {
        return String::new();
    }
    let params = method
        .args
        .iter()
        .map(|arg| format!("{}: {}", arg.name, arg.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!(", {params}")
}

fn render_proxy_args_json(method: &SourceMethod) -> String {
    if method.args.is_empty() {
        return "serde_json::json!({})".to_string();
    }
    let entries = method
        .args
        .iter()
        .map(|arg| format!("{:?}: {}", arg.name, render_proxy_arg_json_expr(arg)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("serde_json::json!({{{entries}}})")
}

fn render_proxy_arg_json_expr(arg: &SourceArg) -> String {
    if arg.ty == "&std::path::Path" {
        format!("{}.to_string_lossy().to_string()", arg.name)
    } else {
        arg.name.clone()
    }
}

fn render_call_arm(method: &SourceMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    match method.call_protocol() {
        Some(CallProtocol::Unit) => format!(
            "        {:?} => {{\n{}            object.{}({}){};\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::ResultUnit) => format!(
            "        {:?} => {{\n{}            object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::Value(_)) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){})\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::ResultValue(_)) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        None => String::new(),
    }
}

fn render_watch_snapshot_arm(method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let value_expr = match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible: true } => format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonFlow { fallible: false } => format!(
            "object.{}({}).first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonState { fallible: true } => format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.value()",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonState { fallible: false } => {
            format!("object.{}({}).value()", method.name, call_args)
        }
        WatchStreamProtocol::TextEvent { .. } => return String::new(),
    };
    format!(
        "        {:?} => {{\n{}            to_core_value({})\n        }}\n",
        method.name, args, value_expr
    )
}

fn render_watch_stream_arm(method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible } => {
            render_json_flow_watch_stream_arm(method, fallible)
        }
        WatchStreamProtocol::JsonState { fallible } => {
            render_json_state_watch_stream_arm(method, fallible)
        }
        WatchStreamProtocol::TextEvent { optional } => {
            render_text_event_watch_stream_arm(method, optional)
        }
    }
}

fn render_json_flow_watch_stream_arm(method: &SourceMethod, fallible: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let flow_expr = if fallible {
        format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let flow = {};\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            spawn_core_task(move || {{\n                let isFirstEvent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));\n                let _ = flow.collect(|value| {{\n                    let kind = if isFirstEvent.swap(false, std::sync::atomic::Ordering::SeqCst) {{\n                        operit_link::CoreEventKind::Snapshot\n                    }} else {{\n                        operit_link::CoreEventKind::Changed\n                    }};\n                    if let Ok(value) = serde_json::to_value(value) {{\n                        let _ = sender.send(operit_link::CoreEvent {{\n                            requestId: Some(requestId.clone()),\n                            targetPath: targetPath.clone(),\n                            propertyName: propertyName.clone(),\n                            kind,\n                            value,\n                        }});\n                    }}\n                }});\n            }});\n            Ok(receiver)\n        }}\n",
        method.name, args, flow_expr
    )
}

fn render_json_state_watch_stream_arm(method: &SourceMethod, fallible: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let state_expr = if fallible {
        format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let stateFlow = {};\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            let isFirstEvent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));\n            let isFirstEventForSubscriber = isFirstEvent.clone();\n            stateFlow.subscribe(move |value| {{\n                let kind = if isFirstEventForSubscriber.swap(false, std::sync::atomic::Ordering::SeqCst) {{\n                    operit_link::CoreEventKind::Snapshot\n                }} else {{\n                    operit_link::CoreEventKind::Changed\n                }};\n                if let Ok(value) = serde_json::to_value(value) {{\n                    let _ = sender.send(operit_link::CoreEvent {{\n                        requestId: Some(requestId.clone()),\n                        targetPath: targetPath.clone(),\n                        propertyName: propertyName.clone(),\n                        kind,\n                        value,\n                    }});\n                }}\n            }});\n            Ok(receiver)\n        }}\n",
        method.name, args, state_expr
    )
}

fn render_text_event_watch_stream_arm(method: &SourceMethod, optional: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let chat_id_expr = method
        .args
        .iter()
        .find(|arg| arg.name == "chatId" || arg.name == "chat_id")
        .map(|arg| arg.name.clone())
        .unwrap_or_else(|| "\"\".to_string()".to_string());
    let stream_expr = if optional {
        format!(
            "object.{}({}).ok_or_else(|| operit_link::CoreLinkError::watchNotFound(&registryKey))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let streamChatId = {}.clone();\n            let stream = {};\n            Ok(core_text_event_stream(streamChatId, stream, request))\n        }}\n",
        method.name, args, chat_id_expr, stream_expr
    )
}

fn render_arg_decoders(method: &SourceMethod) -> String {
    method
        .args
        .iter()
        .map(|arg| {
            format!(
                "            let {}: {} = decode_core_arg(&mut __core_args, {:?})?;\n",
                arg.name,
                render_arg_decode_type(arg),
                arg.name
            )
        })
        .collect::<String>()
}

fn render_arg_decode_type(arg: &SourceArg) -> String {
    if arg.ty == "&str" {
        "String".to_string()
    } else if arg.ty == "Option<&str>" {
        "Option<String>".to_string()
    } else if let Some(inner) =
        single_generic_arg(&arg.ty, "Option").and_then(|inner| inner.strip_prefix('&'))
    {
        format!("Option<{inner}>")
    } else if arg.ty == "&std::path::Path" {
        "String".to_string()
    } else if let Some(inner) = borrowed_slice_inner(&arg.ty) {
        match inner {
            "std::path::PathBuf" => "Vec<std::path::PathBuf>".to_string(),
            "i64" => "Vec<i64>".to_string(),
            "String" => "Vec<String>".to_string(),
            _ => arg.ty.clone(),
        }
    } else if let Some(inner) = arg.ty.strip_prefix('&') {
        inner.to_string()
    } else {
        arg.ty.clone()
    }
}

fn render_arg_call_list(method: &SourceMethod) -> String {
    method
        .args
        .iter()
        .map(render_arg_call_expr)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_arg_call_expr(arg: &SourceArg) -> String {
    if arg.ty == "&str" {
        format!("{}.as_str()", arg.name)
    } else if arg.ty == "Option<&str>" {
        format!("{}.as_deref()", arg.name)
    } else if single_generic_arg(&arg.ty, "Option")
        .and_then(|inner| inner.strip_prefix('&'))
        .is_some()
    {
        format!("{}.as_ref()", arg.name)
    } else if arg.ty == "&std::path::Path" {
        format!("std::path::Path::new(&{})", arg.name)
    } else if borrowed_slice_inner(&arg.ty).is_some() {
        format!("{}.as_slice()", arg.name)
    } else if arg.ty.strip_prefix('&').is_some() {
        format!("&{}", arg.name)
    } else {
        arg.name.clone()
    }
}

fn await_suffix(method: &SourceMethod) -> &'static str {
    if method.is_async {
        ".await"
    } else {
        ""
    }
}

fn json_string(value: &str) -> String {
    serde_json_escape(value)
}

fn option_json_string(value: Option<&str>) -> String {
    match value {
        Some(value) => serde_json_escape(value),
        None => "null".to_string(),
    }
}

fn serde_json_escape(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result.push('"');
    result
}
