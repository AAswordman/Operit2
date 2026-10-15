//! ToolPkg duplex facade over the existing authenticated Watch and Push routes.
use super::device_tools::CoreDeviceToolRuntime;
use operit_link::{CoreCallRequest, CoreEventStream, CoreLinkPushSession, CorePushRequest, CoreValue, CoreWatchRequest};
use operit_plugin_sdk::javascript::{JsDeviceToolCall, JsToolCallRequest, JsToolCallResult, JsToolCallResultData};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot, watch};

type Reply = oneshot::Sender<Result<Value, String>>;
#[derive(Clone)]
pub(super) struct Channel {
    owner: String,
    node: String,
    send: mpsc::Sender<(Option<CoreValue>, Reply)>,
    receive: mpsc::Sender<Reply>,
    cancel: watch::Sender<bool>,
}
pub(super) type Channels = Arc<ChannelRegistry>;
#[derive(Default)]
pub(super) struct ChannelRegistry(Mutex<HashMap<String, Channel>>);
impl Drop for ChannelRegistry {
    fn drop(&mut self) {
        for channel in self.0.get_mut().unwrap().values() { let _ = channel.cancel.send(true); }
    }
}

// Every value has a transport tag so vendor objects cannot collide with byte markers.
fn decode(value: Value) -> Result<CoreValue, String> {
    let kind = value["kind"].as_str().ok_or("Invalid transport value")?;
    let data = value.get("value").cloned().unwrap_or(Value::Null);
    Ok(match kind {
        "null" => CoreValue::Null,
        "bytes" => CoreValue::Bytes(serde_json::from_value::<Vec<u8>>(data).map_err(|_| "Invalid transport bytes")?.into()),
        "list" => CoreValue::List(data.as_array().ok_or("Invalid transport list")?.iter().cloned().map(decode).collect::<Result<_, _>>()?),
        "map" => CoreValue::Map(data.as_object().ok_or("Invalid transport map")?.iter().map(|(k,v)| Ok((k.clone(),decode(v.clone())?))).collect::<Result<_,String>>()?),
        "bool" if data.is_boolean() => operit_link::toCoreValue(data).map_err(|e|e.to_string())?,
        "number" if data.is_number() => operit_link::toCoreValue(data).map_err(|e|e.to_string())?,
        "string" if data.is_string() => operit_link::toCoreValue(data).map_err(|e|e.to_string())?,
        _ => return Err("Invalid transport value".into()),
    })
}
fn encode(value: CoreValue) -> Value {
    match value {
        CoreValue::Null => json!({"kind":"null"}),
        CoreValue::Bool(v) => json!({"kind":"bool","value":v}),
        CoreValue::Signed(v) => json!({"kind":"number","value":v}),
        CoreValue::Unsigned(v) => json!({"kind":"number","value":v}),
        CoreValue::Float(v) => json!({"kind":"number","value":v}),
        CoreValue::String(v) => json!({"kind":"string","value":v}),
        CoreValue::Bytes(v) => json!({"kind":"bytes","value":v.as_ref()}),
        CoreValue::List(v) => json!({"kind":"list","value":v.into_iter().map(encode).collect::<Vec<_>>()}),
        CoreValue::Map(v) => json!({"kind":"map","value":v.into_iter().map(|(k,v)|(k,encode(v))).collect::<serde_json::Map<_,_>>()}),
    }
}

struct ChannelGuard { id: String, channels: std::sync::Weak<ChannelRegistry> }
impl Drop for ChannelGuard {
    fn drop(&mut self) { if let Some(channels) = self.channels.upgrade() {channels.0.lock().unwrap().remove(&self.id);} }
}
async fn run_channel(
    input: Box<dyn CoreLinkPushSession>, mut output: CoreEventStream,
    mut send: mpsc::Receiver<(Option<CoreValue>, Reply)>, mut receive: mpsc::Receiver<Reply>,
    mut cancel: watch::Receiver<bool>,
) {
    let sending = async {
        let mut input=Some(input);
        while let Some((value, reply)) = send.recv().await {
            let result = match value {
                Some(value) => match input.as_mut() {Some(input) => input.send(value).await, None => Err(operit_link::CoreLinkError::internal("Channel input is finished"))},
                None => match input.take() {Some(input) => input.close().await, None => Ok(())},
            }.map(|_| Value::Null).map_err(|e|e.to_string());
            let failed = result.is_err();
            let _ = reply.send(result);
            if failed { break; }
        }
    };
    let receiving = async {
        let mut completed=false;
        while let Some(reply) = receive.recv().await {
            // Keep remote errors as errors; EOF is only a transport end.
            let result = match if completed {None} else {output.recv().await} {
                Some(event) => {completed=event.kind==operit_link::CoreEventKind::Completed; Ok(json!({"done":false,"kind":event.kind,"value":encode(event.value)}))},
                None => Ok(json!({"done":true})),
            };
            let ended = result.as_ref().map(|v|v["done"]==true).unwrap_or(true);
            let _ = reply.send(result);
            if ended {completed=true;}
        }
    };
    tokio::select! {
        _ = cancel.changed() => {},
        _ = sending => {},
        _ = receiving => {},
    }
}

impl CoreDeviceToolRuntime {
    pub(super) fn close_transport_handle(&self, handle: &str) {
        if let Some(channel)=self.channels.0.lock().unwrap().remove(handle) {let _=channel.cancel.send(true);}
    }
    pub(super) fn close_transport_owner(&self, owner: &str) {
        self.channels.0.lock().unwrap().retain(|_, channel| {
            if channel.owner==owner {let _=channel.cancel.send(true);false} else {true}
        });
    }

    pub(super) async fn execute_transport(&self, request: JsToolCallRequest, device: JsDeviceToolCall) -> Result<JsToolCallResult,String> {
        let p = request.parameters;
        let op = p.get("op").and_then(Value::as_str).ok_or("Missing transport operation")?;
        let owner = p.get("__transport_owner").and_then(Value::as_str).ok_or("Missing transport owner")?;
        let value = match op {
            "call" | "open" => {
                let router = self.router.upgrade().ok_or("Owning Core runtime is no longer available")?;
                let target = p.get("target").and_then(Value::as_str).filter(|s|!s.is_empty()&&s.len()<=256).ok_or("Invalid transport target")?;
                let args = decode(p.get("args").cloned().ok_or("Missing transport args")?)?;
                let method = p.get("method").and_then(Value::as_str).filter(|s|!s.is_empty()&&s.len()<=128).ok_or("Invalid transport method")?;
                if op=="call" {
                    let result = router.callNode(device.node_id, CoreCallRequest::new(operit_link::nextCoreRouteRequestId("transport"),target,method,args)).await.result.map_err(|e|e.to_string())?;
                    encode(result)
                } else {
                    if self.channels.0.lock().unwrap().len()>=8 { return Err("Too many open device channels".into()); }
                    let property = p.get("property").and_then(Value::as_str).filter(|s|!s.is_empty()&&s.len()<=128).ok_or("Invalid transport property")?;
                    let output = router.watchNode(device.node_id.clone(), CoreWatchRequest {requestId:operit_link::CoreRequestId(operit_link::nextCoreRouteRequestId("transport")),target:target.into(),propertyName:property.into(),args:args.clone()}).await.map_err(|e|e.to_string())?;
                    let input = router.openPushNode(device.node_id.clone(),CorePushRequest::new(operit_link::nextCoreRouteRequestId("transport"),target,method).withArgs(args)).await.map_err(|e|e.to_string())?;
                    let id = operit_link::nextCoreRouteRequestId("channel");
                    let (send, sending) = mpsc::channel(1);
                    let (receive, receiving) = mpsc::channel(1);
                    let (cancel, cancelled) = watch::channel(false);
                    {
                        let mut channels = self.channels.0.lock().unwrap();
                        if channels.len()>=8 { return Err("Too many open device channels".into()); }
                        channels.insert(id.clone(), Channel {owner:owner.into(),node:device.node_id,send,receive,cancel});
                    }
                    let guard = ChannelGuard {id:id.clone(),channels:Arc::downgrade(&self.channels)};
                    self.scheduler.scheduleHostRuntimeAsyncTask("device-duplex-channel",Box::new(move || Box::pin(async move {
                        let _guard=guard;
                        run_channel(input,output,sending,receiving,cancelled).await;
                    }))).map_err(|e|e.to_string())?;
                    json!({"handle":id})
                }
            }
            "send" | "finish" | "next" | "close" => {
                let id = p.get("handle").and_then(Value::as_str).ok_or("Missing channel handle")?;
                let channel = {
                    let mut channels=self.channels.0.lock().unwrap();
                    let Some(channel)=channels.get(id) else {
                        if op=="close" {return Ok(JsToolCallResult{success:true,..Default::default()});}
                        return Err("Device channel is closed".into());
                    };
                    if channel.owner!=owner || channel.node!=device.node_id {return Err("Device channel owner mismatch".into());}
                    if op=="close" {channels.remove(id).unwrap()} else {channel.clone()}
                };
                if op=="close" {let _=channel.cancel.send(true);Value::Null} else {
                    let (reply,receive)=oneshot::channel();
                    if op=="send" || op=="finish" {
                        let value=if op=="finish" {None} else {Some(decode(p.get("value").cloned().ok_or("Missing transport value")?)?)};
                        channel.send.try_send((value,reply)).map_err(|_|"Device channel send is busy or closed")?;
                    } else {channel.receive.try_send(reply).map_err(|_|"Device channel receive is busy or closed")?;}
                    receive.await.map_err(|_|"Device channel was closed")??
                }
            }
            _ => return Err("Unknown transport operation".into()),
        };
        Ok(JsToolCallResult{success:true,data:JsToolCallResultData::Value(value),error:None})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values_round_trip_with_nested_binary_and_vendor_keys() {
        let original=CoreValue::Map(std::collections::BTreeMap::from([
            ("kind".into(),CoreValue::String("bytes".into())),
            ("payload".into(),CoreValue::List(vec![CoreValue::Bytes(vec![0,255,1].into()),CoreValue::Null])),
        ]));
        assert_eq!(decode(encode(original.clone())).unwrap(),original);
        assert!(decode(json!({"kind":"bytes","value":[256]})).is_err());
    }
}

#[cfg(test)]
mod duplex_tests {
    use super::*;
    use async_trait::async_trait;
    use operit_link::{CoreEvent, CoreEventKind, CoreLinkError, CoreRequestId};
    struct Echo(mpsc::Sender<CoreEvent>);
    #[async_trait]
    impl CoreLinkPushSession for Echo {
        async fn send(&mut self, value: CoreValue) -> Result<(),CoreLinkError> {
            self.0.send(CoreEvent{requestId:Some(CoreRequestId("echo".into())),target:"vendor".into(),propertyName:"output".into(),kind:CoreEventKind::Changed,value}).await.map_err(|_|CoreLinkError::internal("closed"))
        }
        async fn close(self: Box<Self>) -> Result<(),CoreLinkError> {
            self.0.send(CoreEvent{requestId:Some(CoreRequestId("echo".into())),target:"vendor".into(),propertyName:"output".into(),kind:CoreEventKind::Completed,value:CoreValue::Null}).await.map_err(|_|CoreLinkError::internal("closed"))
        }
    }
    #[tokio::test]
    async fn pending_receive_allows_binary_send_and_half_close_preserves_final_output() {
        let (events,output)=CoreEventStream::boundedChannel(1);
        let (send,sending)=mpsc::channel(1);
        let (receive,receiving)=mpsc::channel(1);
        let (cancel,cancelled)=watch::channel(false);
        let task=tokio::spawn(run_channel(Box::new(Echo(events)),output,sending,receiving,cancelled));
        let (read_reply,read)=oneshot::channel();
        receive.send(read_reply).await.unwrap();
        tokio::task::yield_now().await;
        let bytes=CoreValue::Bytes(vec![0,128,255].into());
        let (write_reply,written)=oneshot::channel();
        send.send((Some(bytes.clone()),write_reply)).await.unwrap();
        let result=tokio::time::timeout(std::time::Duration::from_secs(1),read).await.unwrap().unwrap().unwrap();
        assert_eq!(result["kind"],"Changed");
        assert_eq!(decode(result["value"].clone()).unwrap(),bytes);
        assert!(written.await.unwrap().is_ok());
        let (finish_reply,finished)=oneshot::channel();
        send.send((None,finish_reply)).await.unwrap();
        assert!(finished.await.unwrap().is_ok());
        let (reply,read)=oneshot::channel();receive.send(reply).await.unwrap();
        assert_eq!(read.await.unwrap().unwrap()["kind"],"Completed");
        let (reply,read)=oneshot::channel();receive.send(reply).await.unwrap();
        assert_eq!(read.await.unwrap().unwrap()["done"],true);
        cancel.send(true).unwrap();task.await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_wakes_pending_receive_and_runtime_drop_cancels_channels() {
        let (events,output)=CoreEventStream::boundedChannel(1);
        let (send,sending)=mpsc::channel(1);
        let (receive,receiving)=mpsc::channel(1);
        let (cancel,cancelled)=watch::channel(false);
        let task=tokio::spawn(run_channel(Box::new(Echo(events)),output,sending,receiving,cancelled));
        let (reply,read)=oneshot::channel();receive.send(reply).await.unwrap();
        let registry=ChannelRegistry::default();
        registry.0.lock().unwrap().insert("channel".into(),Channel{owner:"engine".into(),node:"device".into(),send,receive,cancel});
        drop(registry);
        assert!(tokio::time::timeout(std::time::Duration::from_secs(1),read).await.unwrap().is_err());
        task.await.unwrap();
    }
}
