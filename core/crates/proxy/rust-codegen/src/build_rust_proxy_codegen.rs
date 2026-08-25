use super::build_rust_codegen_utils::*;
use super::*;

pub(crate) fn render_generated_proxy(objects: &[SourceObject]) -> String {
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
    output.push_str("    #[cfg(not(target_arch = \"wasm32\"))]\n");
    output.push_str("    pub async fn runCoreCommand(&mut self, args: &[String]) -> Result<operit_command_core::CoreCommandOutput, operit_link::CoreLinkError> {\n");
    output.push_str("        let args = operit_rslink_runtime::to_core_arg_value(std::collections::BTreeMap::from([(\"args\", args)]))?;\n");
    let application_id = objects
        .iter()
        .find(|object| object.schema_key == "application")
        .expect("application object must be generated")
        .object_id;
    output.push_str(&format!("        let response = self.client.call(operit_link::CoreCallRequest::new(operit_rslink_runtime::generated_proxy_request_id(), {application_id}, \"runCoreCommand\", args)).await;\n"));
    output.push_str("        let value = response.result?;\n");
    output.push_str("        operit_rslink_runtime::from_core_response_value(value)\n");
    output.push_str("    }\n\n");
    for object in objects
        .iter()
        .filter(|object| !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        let proxy_type = proxy_object_type_name(object);
        if object.access == ObjectAccess::StringNewConstruct {
            output.push_str(&format!(
                "    pub fn {}(&mut self, instanceId: &str) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            output.push_str("        let object_args = std::collections::BTreeMap::from([(\"__core_instance_id\".to_string(), operit_link::CoreValue::String(instanceId.to_string()))]);\n");
            output.push_str(&format!(
                "        {}::new(&mut self.client, {}, object_args)\n",
                proxy_type, object.object_id
            ));
        } else {
            output.push_str(&format!(
                "    pub fn {}(&mut self) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            output.push_str(&format!(
                "        {}::new(&mut self.client, {}, std::collections::BTreeMap::new())\n",
                proxy_type,
                object.object_id
            ));
        }
        output.push_str("    }\n\n");
    }
    output.push_str("}\n\n");

    for object in objects {
        let proxy_type = proxy_object_type_name(object);
        output.push_str(&format!("pub struct {}<'a, C> {{\n", proxy_type));
        output.push_str("    client: &'a mut C,\n");
        output.push_str("    target_object_id: u32,\n");
        output.push_str("    object_args: std::collections::BTreeMap<String, operit_link::CoreValue>,\n");
        output.push_str("}\n\n");
        output.push_str(&format!(
            "impl<'a, C: operit_link::CoreLinkClient> {}<'a, C> {{\n",
            proxy_type
        ));
        output.push_str(
            "    fn new(client: &'a mut C, target_object_id: u32, object_args: std::collections::BTreeMap<String, operit_link::CoreValue>) -> Self {\n",
        );
        output.push_str("        Self { client, target_object_id, object_args }\n");
        output.push_str("    }\n\n");
        output.push_str(
            "    /// Returns mutable access to the link client behind this generated proxy.\n",
        );
        output.push_str("    pub fn generatedClientMut(&mut self) -> &mut C {\n");
        output.push_str("        self.client\n");
        output.push_str("    }\n\n");
        output.push_str("    /// Merges constructor-bound object arguments into one generated request payload.\n");
        output.push_str("    fn generatedArgs(&self, args: operit_link::CoreValue) -> operit_link::CoreValue {\n");
        output.push_str("        let mut values = match args { operit_link::CoreValue::Map(values) => values, value => return value };\n");
        output.push_str("        for (key, value) in &self.object_args { values.entry(key.clone()).or_insert_with(|| value.clone()); }\n");
        output.push_str("        operit_link::CoreValue::Map(values)\n");
        output.push_str("    }\n\n");
        if object.has_proxy_value_call_methods() {
            output.push_str("    async fn callGenerated<T: serde::de::DeserializeOwned>(&mut self, methodName: &str, args: operit_link::CoreValue) -> Result<T, operit_link::CoreLinkError> {\n");
            output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, methodName, self.generatedArgs(args))).await;\n");
            output.push_str("        let value = response.result?;\n");
            output.push_str("        operit_rslink_runtime::from_core_response_value(value)\n");
            output.push_str("    }\n\n");
        }
        if object.has_proxy_unit_call_methods() {
            output.push_str("    async fn callGeneratedUnit(&mut self, methodName: &str, args: operit_link::CoreValue) -> Result<(), operit_link::CoreLinkError> {\n");
            output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, methodName, self.generatedArgs(args))).await;\n");
            output.push_str("        response.result.map(|_| ())\n");
            output.push_str("    }\n\n");
        }
        if object.has_proxy_snapshot_watch_methods() {
            output.push_str("    async fn watchGenerated<T: serde::de::DeserializeOwned>(&mut self, propertyName: &str, args: operit_link::CoreValue) -> Result<T, operit_link::CoreLinkError> {\n");
            output.push_str("        let event = self.client.watchSnapshot(operit_link::CoreWatchRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, propertyName, self.generatedArgs(args))).await?;\n");
            output.push_str("        operit_rslink_runtime::from_core_response_value(event.value)\n");
            output.push_str("    }\n\n");
        }
        for method in object
            .methods
            .iter()
            .filter(|method| method.factory_protocol().is_some())
        {
            output.push_str(&render_proxy_factory_method(objects, method));
        }
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
        for method in object
            .methods
            .iter()
            .filter(|method| method.reverse_stream_protocol().is_some())
        {
            output.push_str(&render_proxy_reverse_stream_method(method));
        }
        output.push_str(&render_proxy_watch_all_method(object));
        output.push_str("}\n\n");
    }
    output
}

/// Renders one typed caller-to-runtime stream method for Rust proxy consumers.
fn render_proxy_reverse_stream_method(method: &SourceMethod) -> String {
    let protocol = method
        .reverse_stream_protocol()
        .expect("reverse stream protocol");
    let params = method
        .args
        .iter()
        .map(|arg| {
            if arg.name == protocol.argument_name {
                format!("mut {}: {}", arg.name, arg.ty)
            } else {
                format!("{}: {}", arg.name, arg.ty)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params = if params.is_empty() {
        String::new()
    } else {
        format!(", {params}")
    };
    let entries = method
        .args
        .iter()
        .filter(|arg| arg.name != protocol.argument_name)
        .map(|arg| {
            format!(
                "__core_args.insert({:?}.to_string(), {});",
                arg.name,
                render_proxy_arg_core_value_expr(arg),
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let args = format!(
        "{{ let mut __core_args = std::collections::BTreeMap::new(); {entries} operit_link::CoreValue::Map(__core_args) }}"
    );
    let input = &protocol.argument_name;
    let mut output = render_cfg_attrs(method);
    output.push_str(&render_doc_comments(method));
    output.push_str(&format!(
        "    pub async fn {}(&mut self{}) -> Result<(), operit_link::CoreLinkError> {{\n",
        method.name, params
    ));
    output.push_str(&format!(
        "        let request = operit_link::CorePushRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, {:?}).withArgs(self.generatedArgs({}));\n",
        method.name, args
    ));
    output.push_str("        let mut sink = self.client.openPush(request).await?;\n");
    output.push_str(&format!(
        "        while let Some(item) = {}.recv().await {{\n",
        input
    ));
    output.push_str("            let value = operit_rslink_runtime::to_core_arg_value(item)?;\n");
    output.push_str("            sink.send(value).await?;\n");
    output.push_str("        }\n");
    output.push_str("        sink.close().await\n");
    output.push_str("    }\n\n");
    output
}

fn proxy_object_type_name(object: &SourceObject) -> String {
    proxy_object_type_name_from_schema_key(&object.schema_key)
}

fn proxy_object_type_name_from_schema_key(schema_key: &str) -> String {
    let mut out = String::from("GeneratedCoreProxy");
    let dispatch_name = dispatch_name_from_schema_key(schema_key);
    for part in dispatch_name.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn render_proxy_factory_method(objects: &[SourceObject], method: &SourceMethod) -> String {
    let factory = method.factory_protocol().expect("factory protocol");
    let proxy_type = proxy_object_type_name_from_schema_key(&factory.target_schema_key);
    let target_object_id = objects
        .iter()
        .find(|object| object.schema_key == factory.target_schema_key)
        .expect("factory target object must be generated")
        .object_id;
    let params = render_proxy_params(method);
    let mut output = render_cfg_attrs(method);
    output.push_str(&render_doc_comments(method));
    output.push_str(&format!(
        "    pub fn {}(&mut self{}) -> {}<'_, C> {{\n",
        method.name, params, proxy_type
    ));
    output.push_str("        let mut object_args = std::collections::BTreeMap::new();\n");
    for (index, arg) in method.args.iter().enumerate() {
        output.push_str(&format!(
            "        object_args.insert(\"__core_factory_arg_{index}\".to_string(), operit_link::CoreValue::String({}.to_string()));\n",
            arg.name
        ));
    }
    output.push_str(&format!(
        "        {}::new(self.client, {}, object_args)\n",
        proxy_type, target_object_id
    ));
    output.push_str("    }\n\n");
    output
}


fn render_proxy_call_method(method: &SourceMethod) -> String {
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    let method_code = match method.call_protocol() {
        Some(CallProtocol::Unit | CallProtocol::ResultUnit { .. }) => format!(
            "    pub async fn {}(&mut self{}) -> Result<(), operit_link::CoreLinkError> {{\n        self.callGeneratedUnit({:?}, {}).await\n    }}\n\n",
            method.name, params, method.name, args_json
        ),
        Some(CallProtocol::Value(value)) => format!(
            "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.callGenerated({:?}, {}).await\n    }}\n\n",
            method.name, params, value, method.name, args_json
        ),
        Some(CallProtocol::ResultValue { value_type, .. }) => format!(
            "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.callGenerated({:?}, {}).await\n    }}\n\n",
            method.name, params, value_type, method.name, args_json
        ),
        None => String::new(),
    };
    render_cfg_attrs(method) + &render_doc_comments(method) + &method_code
}

fn render_proxy_watch_method(object: &SourceObject, method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match &watch.stream {
        WatchStreamProtocol::JsonStream
        | WatchStreamProtocol::StringStream => {
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            let method_code = format!(
                "    pub async fn {}(&mut self{}) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n        self.client.watch(operit_link::CoreWatchRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, {:?}, self.generatedArgs({}))).await\n    }}\n\n",
                method.name, params, method.name, args_json
            );
            render_cfg_attrs(method) + &render_doc_comments(method) + &method_code
        }
        WatchStreamProtocol::JsonFlow { .. } | WatchStreamProtocol::JsonState { .. } => {
            let Some(value) = watch.snapshot_type.as_ref() else {
                return String::new();
            };
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            let mut output = render_cfg_attrs(method);
            output.push_str(&render_doc_comments(method));
            output.push_str(&format!(
                "    pub async fn {}Snapshot(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.watchGenerated({:?}, {}).await\n    }}\n\n",
                method.name, params, value, method.name, args_json
            ));
            let Some(alias) = method.name.strip_suffix("Flow") else {
                return output;
            };
            if alias.is_empty() || object.methods.iter().any(|existing| existing.name == alias) {
                return output;
            }
            output.push_str(&render_cfg_attrs(method));
            output.push_str(&render_alias_doc_comments(method, alias));
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
        .map(|method| {
            format!(
                "{}        propertyNames.push({});\n",
                render_cfg_attrs(method),
                json_string(&method.name)
            )
        })
        .collect::<Vec<_>>();
    if watchable.is_empty() {
        return "    /// Watches every generated state-flow property on this proxy object.\n    pub async fn watchAllGeneratedStateFlows(&mut self, _sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {\n        Ok(())\n    }\n\n".to_string();
    }
    format!(
        "    /// Watches every generated state-flow property on this proxy object.\n    pub async fn watchAllGeneratedStateFlows(&mut self, sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {{\n        let mut propertyNames: Vec<&'static str> = Vec::new();\n{}        for propertyName in propertyNames {{\n            let request = operit_link::CoreWatchRequest::new(operit_rslink_runtime::generated_proxy_request_id(), self.target_object_id, propertyName, self.generatedArgs(operit_link::CoreValue::emptyMap()));\n            let stream = self.client.watch(request).await?;\n            operit_rslink_runtime::forward_core_event_stream(stream, sender.clone(), \"core-proxy-state-flow\")?;\n        }}\n        Ok(())\n    }}\n\n",
        watchable.join("")
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
        return "operit_link::CoreValue::emptyMap()".to_string();
    }
    let entries = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "__core_args.insert({:?}.to_string(), {});",
                arg.name,
                render_proxy_arg_core_value_expr(arg),
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{{ let mut __core_args = std::collections::BTreeMap::new(); {entries} operit_link::CoreValue::Map(__core_args) }}")
}

fn render_proxy_arg_core_value_expr(arg: &SourceArg) -> String {
    if arg.ty == "Vec<u8>" {
        format!("operit_link::CoreValue::Bytes({})", arg.name)
    } else if arg.ty == "&std::path::Path" {
        format!("operit_rslink_runtime::to_named_core_arg_value({:?}, {}.to_string_lossy().to_string())?", arg.name, arg.name)
    } else {
        format!("operit_rslink_runtime::to_named_core_arg_value({:?}, {})?", arg.name, arg.name)
    }
}

fn render_cfg_attrs(method: &SourceMethod) -> String {
    method
        .cfg_attrs
        .iter()
        .map(|attr| format!("    {attr}\n"))
        .collect()
}

fn render_doc_comments(method: &SourceMethod) -> String {
    if method.doc_lines.is_empty() {
        return format!("    /// Generated proxy for `{}`.\n", method.name);
    }
    method
        .doc_lines
        .iter()
        .map(|line| format!("    ///{}\n", doc_comment_suffix(line)))
        .collect()
}

fn render_alias_doc_comments(method: &SourceMethod, alias: &str) -> String {
    if method.doc_lines.is_empty() {
        return format!(
            "    /// Generated proxy alias `{alias}` for `{}`.\n",
            method.name
        );
    }
    let mut output = format!("    /// Alias for `{}`.\n", method.name);
    output.push_str(&render_doc_comments(method));
    output
}

fn doc_comment_suffix(line: &str) -> String {
    if line.is_empty() {
        String::new()
    } else {
        format!(" {line}")
    }
}



