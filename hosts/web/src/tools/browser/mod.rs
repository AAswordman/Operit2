use js_sys::Reflect;
use operit_host_api::{HostResult, WebVisitHost, WebVisitResult, WebVisitRequest};
use wasm_bindgen::prelude::*;

use crate::common::{
    call_web_visit, js_error, js_string_array, js_string_pairs, js_visit_links,
    read_string_property, web_visit_request_to_js,
};

#[derive(Clone, Debug, Default)]
pub struct WebWebVisitHost;

unsafe impl Send for WebWebVisitHost {}
unsafe impl Sync for WebWebVisitHost {}

impl WebWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl WebVisitHost for WebWebVisitHost {
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        let value = call_web_visit("visitWeb", &[web_visit_request_to_js(&request)])?;
        Ok(WebVisitResult {
            url: read_string_property(&value, "url")?,
            title: read_string_property(&value, "title")?,
            content: read_string_property(&value, "content")?,
            metadata: js_string_pairs(
                Reflect::get(&value, &JsValue::from_str("metadata")).map_err(js_error)?,
                "metadata",
            )?,
            links: js_visit_links(Reflect::get(&value, &JsValue::from_str("links")).map_err(js_error)?)?,
            imageLinks: js_string_array(
                Reflect::get(&value, &JsValue::from_str("imageLinks")).map_err(js_error)?,
                "imageLinks",
            )?,
        })
    }
}
