use std::cell::RefCell;

use operit_host_api::{
    HostError, HostResult, HostRuntimeEventHost, HostRuntimeEventRegistration, HostRuntimeEventSink,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

struct WebRuntimeEventState {
    online: Closure<dyn FnMut(web_sys::Event)>,
    offline: Closure<dyn FnMut(web_sys::Event)>,
}

thread_local! {
    static WEB_RUNTIME_EVENT_STATE: RefCell<Option<WebRuntimeEventState>> = const { RefCell::new(None) };
}

/// Streams browser connectivity events into the normalized runtime event ingress.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebHostRuntimeEventHost;

impl WebHostRuntimeEventHost {
    /// Creates the browser runtime event host.
    pub fn new() -> Self {
        Self
    }
}

/// Owns one browser runtime event listener registration.
pub struct WebHostRuntimeEventRegistration;

impl HostRuntimeEventRegistration for WebHostRuntimeEventRegistration {}

impl Drop for WebHostRuntimeEventRegistration {
    /// Removes browser event listeners when Core releases the registration.
    fn drop(&mut self) {
        WEB_RUNTIME_EVENT_STATE.with(|slot| {
            if let Some(state) = slot.borrow_mut().take() {
                let target = browserEventTarget()
                    .expect("browser runtime event target must remain available");
                target
                    .remove_event_listener_with_callback(
                        "online",
                        state.online.as_ref().unchecked_ref(),
                    )
                    .expect("browser online listener must unregister");
                target
                    .remove_event_listener_with_callback(
                        "offline",
                        state.offline.as_ref().unchecked_ref(),
                    )
                    .expect("browser offline listener must unregister");
            }
        });
    }
}

impl HostRuntimeEventHost for WebHostRuntimeEventHost {
    /// Registers online and offline browser event listeners and emits initial connectivity.
    fn startHostRuntimeEventStream(
        &self,
        sink: HostRuntimeEventSink,
    ) -> HostResult<Box<dyn HostRuntimeEventRegistration>> {
        let target = browserEventTarget()?;
        WEB_RUNTIME_EVENT_STATE.with(|slot| -> HostResult<()> {
            if slot.borrow().is_some() {
                return Err(HostError::new(
                    "browser runtime event stream is already registered",
                ));
            }
            let onlineSink = sink.clone();
            let online = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                emitNetworkEvent(&onlineSink, true);
            }) as Box<dyn FnMut(web_sys::Event)>);
            let offlineSink = sink.clone();
            let offline = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                emitNetworkEvent(&offlineSink, false);
            }) as Box<dyn FnMut(web_sys::Event)>);
            target
                .add_event_listener_with_callback("online", online.as_ref().unchecked_ref())
                .map_err(|error| {
                    HostError::new(format!(
                        "register browser online listener failed: {error:?}"
                    ))
                })?;
            target
                .add_event_listener_with_callback("offline", offline.as_ref().unchecked_ref())
                .map_err(|error| {
                    HostError::new(format!(
                        "register browser offline listener failed: {error:?}"
                    ))
                })?;
            *slot.borrow_mut() = Some(WebRuntimeEventState { online, offline });
            Ok(())
        })?;
        emitNetworkEvent(&sink, browserOnline()?);
        Ok(Box::new(WebHostRuntimeEventRegistration))
    }
}

/// Returns the browser-global event target available to windows and workers.
#[allow(non_snake_case)]
fn browserEventTarget() -> HostResult<web_sys::EventTarget> {
    js_sys::global()
        .dyn_into::<web_sys::EventTarget>()
        .map_err(|_| HostError::new("browser global event target is unavailable"))
}

/// Reads the browser-global connectivity state for windows and workers.
#[allow(non_snake_case)]
fn browserOnline() -> HostResult<bool> {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
        .map_err(|error| HostError::new(format!("read browser navigator failed: {error:?}")))?;
    js_sys::Reflect::get(&navigator, &JsValue::from_str("onLine"))
        .map_err(|error| HostError::new(format!("read browser connectivity failed: {error:?}")))?
        .as_bool()
        .ok_or_else(|| HostError::new("browser connectivity state is not boolean"))
}

/// Emits the shared browser network-change structure.
#[allow(non_snake_case)]
fn emitNetworkEvent(sink: &HostRuntimeEventSink, connected: bool) {
    sink(serde_json::json!({
        "domain": "host",
        "source": "web.event",
        "topic": "system.network.changed",
        "platform": "web",
        "payload": {
            "connected": connected,
            "networkType": if connected { "other" } else { "none" },
            "metered": null,
            "interfaceName": null,
        },
        "occurredAtMillis": js_sys::Date::now() as u64,
    }));
}
