use super::build_rust_codegen_utils::*;
use super::build_rust_dispatch_codegen::{
    render_core_proxy_dispatch, render_object_call_dispatch, render_object_sync_call_dispatch,
    render_object_watch_dispatch, render_object_watch_snapshot_dispatch,
};
use super::build_rust_proxy_codegen::render_generated_proxy;
use super::*;

pub(crate) use super::build_rust_schema_codegen::render_schema;

pub(crate) fn render_generated(objects: &[SourceObject], schema_json: &str) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_core_proxy_schema() -> serde_json::Value {\n");
    output.push_str("    serde_json::from_str(r#\"");
    output.push_str(&schema_json);
    output.push_str("\"#).expect(\"generated core proxy schema must be valid JSON\")\n");
    output.push_str("}\n\n");
    for object in objects {
        if object_uses_arc_mutex_instance(&object.access)
            && object
                .methods
                .iter()
                .any(|method| method.is_async && method.call_protocol().is_some())
        {
            panic!(
                "Arc<Mutex<Self>> core proxy object exposes async call methods: {}",
                object.schema_key
            );
        }
        output.push_str(&render_object_call_dispatch(object));
        output.push('\n');
        output.push_str(&render_object_sync_call_dispatch(object));
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
