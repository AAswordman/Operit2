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
    output.push_str("        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), operit_link::CoreObjectPath::parse(\"application\"), \"runCoreCommand\", serde_json::json!({ \"args\": args }))).await;\n");
    output.push_str("        let value = response.result?;\n");
    output.push_str("        serde_json::from_value(value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
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
            .filter(|method| method.factory_protocol().is_some())
        {
            output.push_str(&render_proxy_factory_method(method));
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
        output.push_str(&render_proxy_watch_all_method(object));
        output.push_str("}\n\n");
    }
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

fn render_proxy_factory_method(method: &SourceMethod) -> String {
    let factory = method.factory_protocol().expect("factory protocol");
    let proxy_type = proxy_object_type_name_from_schema_key(&factory.target_schema_key);
    let params = render_proxy_params(method);
    let mut output = render_cfg_attrs(method);
    output.push_str(&format!(
        "    pub fn {}(&mut self{}) -> {}<'_, C> {{\n",
        method.name, params, proxy_type
    ));
    output.push_str("        let mut segments = self.target_path.segments.clone();\n");
    output.push_str(&format!(
        "        segments.push({:?}.to_string());\n",
        method.name
    ));
    for arg in &method.args {
        output.push_str(&format!(
            "        segments.push({}.to_string());\n",
            arg.name
        ));
    }
    output.push_str(&format!(
        "        {}::new(self.client, operit_link::CoreObjectPath {{ segments }})\n",
        proxy_type
    ));
    output.push_str("    }\n\n");
    output
}

fn render_proxy_call_method(method: &SourceMethod) -> String {
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    let method_code = match method.call_protocol() {
        Some(CallProtocol::Unit | CallProtocol::ResultUnit) => format!(
            "    pub async fn {}(&mut self{}) -> Result<(), operit_link::CoreLinkError> {{\n        self.callGeneratedUnit({:?}, {}).await\n    }}\n\n",
            method.name, params, method.name, args_json
        ),
        Some(CallProtocol::Value(value) | CallProtocol::ResultValue(value)) => format!(
            "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.callGenerated({:?}, {}).await\n    }}\n\n",
            method.name, params, value, method.name, args_json
        ),
        None => String::new(),
    };
    render_cfg_attrs(method) + &method_code
}

fn render_proxy_watch_method(object: &SourceObject, method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match &watch.stream {
        WatchStreamProtocol::JsonStream
        | WatchStreamProtocol::StringStream
        | WatchStreamProtocol::TextEvent { .. } => {
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            let method_code = format!(
                "    pub async fn {}(&mut self{}) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n        self.client.watch(operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), {:?}, {})).await\n    }}\n\n",
                method.name, params, method.name, args_json
            );
            render_cfg_attrs(method) + &method_code
        }
        WatchStreamProtocol::JsonFlow { .. } | WatchStreamProtocol::JsonState { .. } => {
            let Some(value) = watch.snapshot_type.as_ref() else {
                return String::new();
            };
            let params = render_proxy_params(method);
            let args_json = render_proxy_args_json(method);
            let mut output = render_cfg_attrs(method);
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
        return "    pub async fn watchAllGeneratedStateFlows(&mut self, _sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {\n        Ok(())\n    }\n\n".to_string();
    }
    format!(
        "    pub async fn watchAllGeneratedStateFlows(&mut self, sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {{\n        let mut propertyNames: Vec<&'static str> = Vec::new();\n{}        for propertyName in propertyNames {{\n            let request = operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), propertyName, serde_json::json!({{}}));\n            let mut stream = self.client.watch(request).await?;\n            let sender = sender.clone();\n            tokio::spawn(async move {{\n                while let Some(event) = stream.recv().await {{\n                    let _ = sender.send(event);\n                }}\n            }});\n        }}\n        Ok(())\n    }}\n\n",
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

fn render_cfg_attrs(method: &SourceMethod) -> String {
    method
        .cfg_attrs
        .iter()
        .map(|attr| format!("    {attr}\n"))
        .collect()
}
