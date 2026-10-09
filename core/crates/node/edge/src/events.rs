//! Reusable event producer boundary; no screen, GPIO or sensor assumptions.
use operit_edge_contract::events::EdgeEventBatch;
use std::{future::Future, pin::Pin, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeEventRoute {
    pub space_id: String,
    pub chat_id: String,
    pub package_name: String,
}
pub type EdgeEventRouteProvider = dyn Fn(&str) -> Option<EdgeEventRoute> + Send + Sync;
#[derive(Clone)]
pub struct EdgeEventDelivery {
    pub route: EdgeEventRoute,
    pub batch: EdgeEventBatch,
}
/// One worker per producer. Implementations snapshot without holding a lock over I/O.
pub trait EdgeEventSource: Send + Sync {
    fn set_event_route_provider(&self, provider: Arc<EdgeEventRouteProvider>);
    fn pending_event_delivery(&self) -> Option<EdgeEventDelivery>;
    fn disable_event_delivery(&self, delivery: &EdgeEventDelivery);
    fn acknowledge_event_delivery(&self, delivery: &EdgeEventDelivery, next: u64) -> bool;
    fn wait_for_event(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Optional reusable producer for GPIO, serial, sensors, buttons, etc. It is not
/// a permission grant: subscribe must be reached through an authenticated action.
pub struct EdgeEventQueue {
    source: String,
    provider: std::sync::Mutex<Option<Arc<EdgeEventRouteProvider>>>,
    state: std::sync::Mutex<Option<QueueState>>,
    changed: tokio::sync::Notify,
}
struct QueueState {
    route: EdgeEventRoute,
    stream: String,
    sequence: u64,
    acknowledged: u64,
    events: std::collections::VecDeque<operit_edge_contract::events::EdgeActionEvent>,
}
impl EdgeEventQueue {
    pub fn new(source: &str) -> Result<Self, &'static str> {
        let valid = !source.is_empty()
            && source.len() <= 108
            && source
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c));
        if !valid {
            return Err("Invalid Edge event source");
        }
        Ok(Self {
            source: source.into(),
            provider: std::sync::Mutex::new(None),
            state: std::sync::Mutex::new(None),
            changed: tokio::sync::Notify::new(),
        })
    }
    /// Capture the current authorized Space/Binding once. Replacing the stream
    /// invalidates old pending deliveries; no disconnected events are inherited.
    pub fn subscribe(&self, package: &str) -> Result<String, &'static str> {
        if package.is_empty()
            || package.len() > 108
            || !package
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
        {
            return Err("Invalid event receiver package");
        }
        let provider = self
            .provider
            .lock()
            .unwrap()
            .clone()
            .ok_or("Event routing unavailable")?;
        let route = provider(package).ok_or("Authorized event Binding unavailable")?;
        let stream = uuid::Uuid::new_v4().to_string();
        *self.state.lock().unwrap() = Some(QueueState {
            route,
            stream: stream.clone(),
            sequence: 0,
            acknowledged: 0,
            events: std::collections::VecDeque::new(),
        });
        self.changed.notify_one();
        Ok(stream)
    }
    pub fn publish(&self, action: &str, data: serde_json::Value) -> Result<(), &'static str> {
        // Bound individual records as well as count. The carrier counts route
        // fields too and shrinks a batch if necessary; no arbitrary binary stream.
        if action.is_empty()
            || action.len() > 64
            || !action
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            || !data.is_object()
            || serde_json::to_vec(&data)
                .map_err(|_| "Invalid event data")?
                .len()
                > 512
        {
            return Err("Invalid or oversized Edge action event");
        }
        let mut state = self.state.lock().unwrap();
        let state = state.as_mut().ok_or("No event receiver")?;
        if state.sequence == operit_edge_contract::events::MAX_SAFE_SEQUENCE {
            return Err("Event stream exhausted");
        }
        let event = operit_edge_contract::events::EdgeActionEvent {
            seq: state.sequence + 1,
            action: action.into(),
            data,
        };
        // Reserve the largest allowed device ID, including all receiver metadata.
        // Reject before queueing if even one event cannot fit the entry envelope.
        let envelope = serde_json::json!({"chatId":state.route.chat_id,"nodeId":"x".repeat(128),
            "packageName":state.route.package_name,"payload":{"v":1,"source":self.source,
            "stream":state.stream,"events":[event],"next":event.seq,"lostBefore":state.sequence}});
        if serde_json::to_vec(&envelope).unwrap().len() > operit_edge_contract::events::MAX_REQUEST
        {
            return Err("Edge event cannot fit the routed request envelope");
        }
        state.sequence += 1;
        if state.events.len() == 16 {
            state.events.pop_front();
        }
        state.events.push_back(event);
        self.changed.notify_one();
        Ok(())
    }
    pub fn unsubscribe(&self, stream: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.as_ref().is_some_and(|s| s.stream == stream) {
            *state = None;
            self.changed.notify_one();
            true
        } else {
            false
        }
    }
}
impl EdgeEventSource for EdgeEventQueue {
    fn set_event_route_provider(&self, provider: Arc<EdgeEventRouteProvider>) {
        *self.provider.lock().unwrap() = Some(provider);
    }
    fn pending_event_delivery(&self) -> Option<EdgeEventDelivery> {
        let state = self.state.lock().unwrap();
        let s = state.as_ref()?;
        let events: Vec<_> = s
            .events
            .iter()
            .filter(|e| e.seq > s.acknowledged)
            .take(operit_edge_contract::events::MAX_BATCH)
            .cloned()
            .collect();
        Some(EdgeEventDelivery {
            route: s.route.clone(),
            batch: EdgeEventBatch {
                v: 1,
                source: self.source.clone(),
                stream: s.stream.clone(),
                next: events.last()?.seq,
                lost_before: s.events.front()?.seq.saturating_sub(1),
                events,
            },
        })
    }
    fn disable_event_delivery(&self, delivery: &EdgeEventDelivery) {
        let mut state = self.state.lock().unwrap();
        if state
            .as_ref()
            .is_some_and(|s| s.stream == delivery.batch.stream && s.route == delivery.route)
        {
            *state = None;
        }
    }
    fn acknowledge_event_delivery(&self, delivery: &EdgeEventDelivery, next: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        let Some(s) = state.as_mut() else {
            return false;
        };
        if delivery.batch.source != self.source
            || delivery.batch.stream != s.stream
            || delivery.route != s.route
            || next != delivery.batch.next
            || next < s.acknowledged
        {
            return false;
        }
        s.acknowledged = next;
        true
    }
    fn wait_for_event(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(self.changed.notified())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn queue() -> EdgeEventQueue {
        let q = EdgeEventQueue::new("sensor.environment").unwrap();
        q.set_event_route_provider(Arc::new(|package| {
            Some(EdgeEventRoute {
                space_id: "space".into(),
                chat_id: "chat".into(),
                package_name: package.into(),
            })
        }));
        q.subscribe("com.example.sensor").unwrap();
        q
    }
    #[test]
    fn non_screen_producer_dispatches_actions_and_guards_old_receivers() {
        let q = queue();
        q.publish("temperature.changed", json!({"celsius":23.5}))
            .unwrap();
        q.publish("serial.message", json!({"text":"hello"}))
            .unwrap();
        let d = q.pending_event_delivery().unwrap();
        assert!(d.batch.valid());
        assert_eq!(d.batch.events[0].action, "temperature.changed");
        assert_eq!(d.batch.events[1].action, "serial.message");
        assert!(!q.acknowledge_event_delivery(&d, 1));
        assert!(q.acknowledge_event_delivery(&d, 2));
        assert!(q.pending_event_delivery().is_none());
        q.subscribe("com.example.other").unwrap();
        assert!(!q.acknowledge_event_delivery(&d, 2));
        q.disable_event_delivery(&d);
        q.publish("gpio.changed", json!({"pin":2,"level":true}))
            .unwrap();
        assert_eq!(
            q.pending_event_delivery().unwrap().route.package_name,
            "com.example.other"
        );
    }
    #[test]
    fn producer_rejects_a_single_event_that_cannot_fit_route_metadata() {
        let q = EdgeEventQueue::new(&"s".repeat(108)).unwrap();
        q.set_event_route_provider(Arc::new(|package| {
            Some(EdgeEventRoute {
                space_id: "space".into(),
                chat_id: "c".repeat(128),
                package_name: package.into(),
            })
        }));
        q.subscribe(&"p".repeat(108)).unwrap();
        assert!(q
            .publish(&"a".repeat(64), json!({"text":"x".repeat(480)}))
            .is_err());
        assert!(q.pending_event_delivery().is_none());
        q.publish("sample", json!({})).unwrap();
        assert_eq!(q.pending_event_delivery().unwrap().batch.next, 1);
    }
    #[test]
    fn producer_bounds_data_reports_loss_and_requires_an_authorized_binding() {
        let q = queue();
        assert!(q.publish("bad action", json!({})).is_err());
        assert!(q
            .publish("sample", json!({"text":"x".repeat(513)}))
            .is_err());
        assert!(q.publish("sample", json!(null)).is_err());
        for n in 0..20 {
            q.publish("sample", json!({"n":n})).unwrap();
        }
        let d = q.pending_event_delivery().unwrap();
        assert_eq!(d.batch.lost_before, 4);
        assert_eq!(d.batch.events.len(), 4);
        assert_eq!(d.batch.events[0].seq, 5);
        assert!(q.unsubscribe(&d.batch.stream));
        assert!(q.publish("sample", json!({})).is_err());
        let q = EdgeEventQueue::new("gpio").unwrap();
        assert!(q.subscribe("com.example.sensor").is_err());
    }
}
