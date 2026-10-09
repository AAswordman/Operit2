//! Generic Edge event ingress uses the existing authorized Space carrier.
#![allow(non_snake_case)]
use super::{spawnNodeUiSubscription, uiTaskCancelled, NodeUiTask, SESSION};
use operit_link::{CoreCallRequest, CORE_INTERNAL_TARGET};
use operit_node_runtime::NodeServices::NodeServices;
use std::sync::{atomic::Ordering, Arc, Mutex};

/// One bounded, event-driven worker shared by firmware and simulator. It never
/// queries Core for input. Pending events alone can cause finite delivery retries.
pub fn startEdgeEvents(
    services: NodeServices,
    source: Arc<dyn operit_node_edge::events::EdgeEventSource>,
    nodeId: String,
) -> NodeUiTask {
    use operit_node_edge::events::EdgeEventRoute;
    let captureServices = services.clone();
    source.set_event_route_provider(Arc::new(move |package| {
        let connection = captureServices.peers().spaceConnection()?;
        let session = SESSION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .clone()?;
        if !session.ready.load(Ordering::Acquire)
            || session.retired.load(Ordering::Acquire)
            || session.spaceIdentity.as_ref() != Some(&connection.identity)
        {
            return None;
        }
        Some(EdgeEventRoute {
            space_id: connection.identity.spaceId,
            chat_id: session.chatId.clone(),
            package_name: package.into(),
        })
    }));
    spawnNodeUiSubscription(move |mut cancelled| async move {
        let mut peers = services.peers().subscribePeerChanges();
        let mut attempts = 0u32;
        let mut lastStream = String::new();
        loop {
            if *cancelled.borrow() {
                return;
            }
            let pending = source.pending_event_delivery();
            if pending.is_none() || attempts >= 5 {
                tokio::select! {
                    _ = uiTaskCancelled(&mut cancelled) => return,
                    _ = source.wait_for_event() => {},
                    result = peers.recv() => {if matches!(result, Err(tokio::sync::broadcast::error::RecvError::Closed)) {return;}},
                }
                attempts = 0;
                continue;
            }
            let mut delivery = pending.unwrap();
            if delivery.batch.stream != lastStream {
                lastStream = delivery.batch.stream.clone();
                attempts = 0;
            }
            let connection = services.peers().spaceConnection();
            let mut accepted = false;
            if let Some(connection) = connection {
                if connection.identity.spaceId != delivery.route.space_id {
                    // A different Space must never inherit this stream's receiver.
                    source.disable_event_delivery(&delivery);
                    continue;
                }
                // Route identifiers count toward the same 1024-byte entry limit.
                let args = loop {
                    let args = serde_json::json!({"chatId":delivery.route.chat_id,"nodeId":nodeId,
                        "packageName":delivery.route.package_name,"payload":delivery.batch});
                    if serde_json::to_vec(&args).unwrap().len()
                        <= operit_edge_contract::events::MAX_REQUEST
                    {
                        break Some(args);
                    }
                    if delivery.batch.events.len() <= 1 {
                        break None;
                    }
                    delivery.batch.events.pop();
                    delivery.batch.next = delivery.batch.events.last().unwrap().seq;
                };
                if let Some(args) = args {
                    if let Ok(args) = operit_link::toCoreValue(args) {
                        let request = CoreCallRequest::new(
                            operit_link::nextCoreRouteRequestId("edge-event"),
                            CORE_INTERNAL_TARGET,
                            operit_edge_contract::events::METHOD,
                            args,
                        );
                        // Do not cancel an in-flight Link transaction; drain its ACK
                        // even on shutdown/entry replacement, as with UI subscriptions.
                        let response = connection.client.call(request).await;
                        if let Ok(value) = response.result {
                            if let Ok(ack) = operit_link::fromCoreValue::<serde_json::Value>(value)
                            {
                                if ack["success"] == true {
                                    accepted = source.acknowledge_event_delivery(
                                        &delivery,
                                        ack["next"].as_u64().unwrap_or(0),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if accepted {
                attempts = 0;
                continue;
            }
            attempts += 1;
            tokio::select! {
                _ = uiTaskCancelled(&mut cancelled) => return,
                _ = source.wait_for_event() => {attempts = 0;},
                result = peers.recv() => {if matches!(result, Err(tokio::sync::broadcast::error::RecvError::Closed)) {return;} attempts = 0;},
                _ = tokio::time::sleep(std::time::Duration::from_millis(1000u64 << (attempts - 1))) => {},
            }
        }
    })
}
