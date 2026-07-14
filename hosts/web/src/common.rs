use std::collections::BTreeMap;

use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use operit_host_api::{
    AppOperationData, BluetoothBleCharacteristicAddress, BluetoothBleCharacteristicData,
    BluetoothBleConnectRequest, BluetoothBleNotificationData, BluetoothBleNotificationEntry,
    BluetoothBleServiceData, BluetoothBleServicesData, BluetoothBleSubscribeRequest,
    BluetoothBleWriteAndReadRequest, BluetoothBleWriteRequest, BluetoothBondedDevicesData,
    BluetoothClassicAcceptRequest, BluetoothClassicConnectRequest, BluetoothClassicListenRequest,
    BluetoothDeviceData, BluetoothPayload, BluetoothReadData, BluetoothReadRequest,
    BluetoothScanRequest, BluetoothScanResultData, BluetoothScannedDeviceData,
    BluetoothSessionData, BluetoothStateData, BluetoothTransferData, FindFilesRequest,
    GrepCodeRequest, HostError, HostResult, HttpFilePart, HttpRequestData, HttpResponseData,
    ManagedRuntimeProgram, MusicPlaybackRequest, MusicPlaybackStatus, RuntimeProcessRequest,
    SqliteRow, SqliteValue, SystemSettingData, WebVisitLinkData, WebVisitRequest,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub(crate) fn call_file_system(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("fileSystem")?;
    call_function(&module, method, args)
}

pub(crate) fn call_web_visit(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("webVisit")?;
    call_function(&module, method, args)
}

pub(crate) fn call_http(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("http")?;
    call_function(&module, method, args)
}

pub(crate) fn call_managed_runtime(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("managedRuntime")?;
    call_function(&module, method, args)
}

pub(crate) fn call_managed_runtime_process(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("managedRuntimeProcess")?;
    call_function(&module, method, args)
}

pub(crate) fn call_storage(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("runtimeStorage")?;
    call_function(&module, method, args)
}

pub(crate) fn call_secret_store(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("hostSecretStore")?;
    call_function(&module, method, args)
}

pub(crate) fn call_sqlite(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("sqlite")?;
    call_function(&module, method, args)
}

pub(crate) fn call_system(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("systemOperation")?;
    call_function(&module, method, args)
}

pub(crate) fn call_tts_playback(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("ttsPlayback")?;
    call_function(&module, method, args)
}

pub(crate) fn call_music_playback(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("musicPlayback")?;
    call_function(&module, method, args)
}

pub(crate) fn call_bluetooth(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("bluetooth")?;
    call_function(&module, method, args)
}

/// Calls one local inference function on the installed JavaScript host bridge.
pub(crate) fn call_local_inference(method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let module = bridge_module("localInference")?;
    call_function(&module, method, args)
}

fn bridge_module(name: &str) -> HostResult<JsValue> {
    let global = js_sys::global();
    let host =
        Reflect::get(global.as_ref(), &JsValue::from_str("__operitHost")).map_err(js_error)?;
    if host.is_null() || host.is_undefined() {
        return Err(HostError::new(
            "web host bridge __operitHost is not installed",
        ));
    }
    let module = Reflect::get(&host, &JsValue::from_str(name)).map_err(js_error)?;
    if module.is_null() || module.is_undefined() {
        return Err(HostError::new(format!(
            "web host bridge module is not installed: {name}"
        )));
    }
    Ok(module)
}

fn call_function(target: &JsValue, method: &str, args: &[JsValue]) -> HostResult<JsValue> {
    let function = Reflect::get(target, &JsValue::from_str(method))
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(|_| {
            HostError::new(format!(
                "web host bridge method is not a function: {method}"
            ))
        })?;
    let array = Array::new();
    for arg in args {
        array.push(arg);
    }
    let value = function.apply(target, &array).map_err(js_error)?;
    if value.is_instance_of::<Promise>() {
        return Err(HostError::new(format!(
            "web host bridge method returned an asynchronous Promise: {method}"
        )));
    }
    Ok(value)
}

pub(crate) fn find_files_request_to_js(request: &FindFilesRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "path", JsValue::from_str(&request.path));
    set_property(&object, "pattern", JsValue::from_str(&request.pattern));
    set_property(
        &object,
        "maxDepth",
        JsValue::from_f64(request.maxDepth as f64),
    );
    set_property(
        &object,
        "usePathPattern",
        JsValue::from_bool(request.usePathPattern),
    );
    set_property(
        &object,
        "caseInsensitive",
        JsValue::from_bool(request.caseInsensitive),
    );
    object.into()
}

pub(crate) fn grep_code_request_to_js(request: &GrepCodeRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "path", JsValue::from_str(&request.path));
    set_property(&object, "pattern", JsValue::from_str(&request.pattern));
    set_property(
        &object,
        "filePattern",
        JsValue::from_str(&request.filePattern),
    );
    set_property(
        &object,
        "caseInsensitive",
        JsValue::from_bool(request.caseInsensitive),
    );
    set_property(
        &object,
        "contextLines",
        JsValue::from_f64(request.contextLines as f64),
    );
    set_property(
        &object,
        "maxResults",
        JsValue::from_f64(request.maxResults as f64),
    );
    object.into()
}

pub(crate) fn web_visit_request_to_js(request: &WebVisitRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "url", JsValue::from_str(&request.url));
    set_property(&object, "headers", string_pairs_to_js(&request.headers));
    set_property(&object, "userAgent", JsValue::from_str(&request.userAgent));
    set_property(
        &object,
        "includeImageLinks",
        JsValue::from_bool(request.includeImageLinks),
    );
    object.into()
}

pub(crate) fn music_playback_request_to_js(request: MusicPlaybackRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "source", JsValue::from_str(&request.source));
    set_property(
        &object,
        "sourceType",
        JsValue::from_str(&request.sourceType),
    );
    set_property(
        &object,
        "title",
        optional_string_to_js(request.title.as_ref()),
    );
    set_property(
        &object,
        "artist",
        optional_string_to_js(request.artist.as_ref()),
    );
    set_property(
        &object,
        "loopPlayback",
        JsValue::from_bool(request.loopPlayback),
    );
    set_property(&object, "volume", JsValue::from_f64(request.volume));
    set_property(
        &object,
        "startPositionMs",
        JsValue::from_f64(request.startPositionMs as f64),
    );
    object.into()
}

pub(crate) fn music_playback_status(value: JsValue) -> HostResult<MusicPlaybackStatus> {
    Ok(MusicPlaybackStatus {
        state: read_string_property(&value, "state")?,
        source: read_optional_string_property(&value, "source")?,
        sourceType: read_optional_string_property(&value, "sourceType")?,
        title: read_optional_string_property(&value, "title")?,
        artist: read_optional_string_property(&value, "artist")?,
        durationMs: read_optional_i64_property(&value, "durationMs")?,
        positionMs: read_i64_property(&value, "positionMs")?,
        bufferedPositionMs: read_i64_property(&value, "bufferedPositionMs")?,
        volume: read_f64_property(&value, "volume")?,
        loopPlayback: read_bool_property(&value, "loopPlayback")?,
        message: read_string_property(&value, "message")?,
    })
}

pub(crate) fn bluetooth_scan_request_to_js(request: BluetoothScanRequest) -> JsValue {
    let object = Object::new();
    set_property(
        &object,
        "durationMs",
        JsValue::from_f64(request.durationMs as f64),
    );
    set_property(
        &object,
        "includeBle",
        JsValue::from_bool(request.includeBle),
    );
    object.into()
}

pub(crate) fn bluetooth_classic_connect_request_to_js(
    request: BluetoothClassicConnectRequest,
) -> JsValue {
    let object = Object::new();
    set_property(&object, "address", JsValue::from_str(&request.address));
    set_property(&object, "uuid", JsValue::from_str(&request.uuid));
    object.into()
}

pub(crate) fn bluetooth_classic_listen_request_to_js(
    request: BluetoothClassicListenRequest,
) -> JsValue {
    let object = Object::new();
    set_property(&object, "name", JsValue::from_str(&request.name));
    set_property(&object, "uuid", JsValue::from_str(&request.uuid));
    object.into()
}

pub(crate) fn bluetooth_classic_accept_request_to_js(
    request: BluetoothClassicAcceptRequest,
) -> JsValue {
    let object = Object::new();
    set_property(
        &object,
        "listenerSessionId",
        JsValue::from_str(&request.listenerSessionId),
    );
    set_property(
        &object,
        "timeoutMs",
        JsValue::from_f64(request.timeoutMs as f64),
    );
    object.into()
}

pub(crate) fn bluetooth_payload_to_js(payload: BluetoothPayload) -> JsValue {
    let object = Object::new();
    set_property(
        &object,
        "text",
        optional_string_to_js(payload.text.as_ref()),
    );
    set_property(
        &object,
        "dataBase64",
        optional_string_to_js(payload.dataBase64.as_ref()),
    );
    object.into()
}

pub(crate) fn bluetooth_read_request_to_js(request: BluetoothReadRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "sessionId", JsValue::from_str(&request.sessionId));
    set_property(
        &object,
        "maxBytes",
        JsValue::from_f64(request.maxBytes as f64),
    );
    set_property(
        &object,
        "timeoutMs",
        JsValue::from_f64(request.timeoutMs as f64),
    );
    object.into()
}

pub(crate) fn bluetooth_ble_connect_request_to_js(request: BluetoothBleConnectRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "address", JsValue::from_str(&request.address));
    set_property(
        &object,
        "autoConnect",
        JsValue::from_bool(request.autoConnect),
    );
    object.into()
}

pub(crate) fn bluetooth_ble_characteristic_address_to_js(
    address: BluetoothBleCharacteristicAddress,
) -> JsValue {
    let object = Object::new();
    set_property(&object, "sessionId", JsValue::from_str(&address.sessionId));
    set_property(
        &object,
        "serviceUuid",
        JsValue::from_str(&address.serviceUuid),
    );
    set_property(
        &object,
        "characteristicUuid",
        JsValue::from_str(&address.characteristicUuid),
    );
    set_property(
        &object,
        "timeoutMs",
        JsValue::from_f64(address.timeoutMs as f64),
    );
    object.into()
}

pub(crate) fn bluetooth_ble_write_request_to_js(request: BluetoothBleWriteRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "sessionId", JsValue::from_str(&request.sessionId));
    set_property(
        &object,
        "serviceUuid",
        JsValue::from_str(&request.serviceUuid),
    );
    set_property(
        &object,
        "characteristicUuid",
        JsValue::from_str(&request.characteristicUuid),
    );
    set_property(
        &object,
        "text",
        optional_string_to_js(request.text.as_ref()),
    );
    set_property(
        &object,
        "dataBase64",
        optional_string_to_js(request.dataBase64.as_ref()),
    );
    object.into()
}

pub(crate) fn bluetooth_ble_write_and_read_request_to_js(
    request: BluetoothBleWriteAndReadRequest,
) -> JsValue {
    let object = Object::new();
    set_property(&object, "sessionId", JsValue::from_str(&request.sessionId));
    set_property(
        &object,
        "writeServiceUuid",
        JsValue::from_str(&request.writeServiceUuid),
    );
    set_property(
        &object,
        "writeCharacteristicUuid",
        JsValue::from_str(&request.writeCharacteristicUuid),
    );
    set_property(
        &object,
        "readServiceUuid",
        JsValue::from_str(&request.readServiceUuid),
    );
    set_property(
        &object,
        "readCharacteristicUuid",
        JsValue::from_str(&request.readCharacteristicUuid),
    );
    set_property(
        &object,
        "text",
        optional_string_to_js(request.text.as_ref()),
    );
    set_property(
        &object,
        "dataBase64",
        optional_string_to_js(request.dataBase64.as_ref()),
    );
    set_property(
        &object,
        "timeoutMs",
        JsValue::from_f64(request.timeoutMs as f64),
    );
    object.into()
}

pub(crate) fn bluetooth_ble_subscribe_request_to_js(
    request: BluetoothBleSubscribeRequest,
) -> JsValue {
    let object = Object::new();
    set_property(&object, "sessionId", JsValue::from_str(&request.sessionId));
    set_property(
        &object,
        "serviceUuid",
        JsValue::from_str(&request.serviceUuid),
    );
    set_property(
        &object,
        "characteristicUuid",
        JsValue::from_str(&request.characteristicUuid),
    );
    set_property(&object, "enable", JsValue::from_bool(request.enable));
    object.into()
}

pub(crate) fn http_request_to_js(request: HttpRequestData) -> JsValue {
    let object = Object::new();
    set_property(&object, "url", JsValue::from_str(&request.url));
    set_property(&object, "method", JsValue::from_str(&request.method));
    set_property(&object, "headers", string_pairs_to_js(&request.headers));
    set_property(&object, "body", bytes_to_js(&request.body));
    set_property(
        &object,
        "formFields",
        string_pairs_to_js(&request.formFields),
    );
    set_property(
        &object,
        "fileParts",
        http_file_parts_to_js(&request.fileParts),
    );
    set_property(
        &object,
        "connectTimeoutSeconds",
        JsValue::from_f64(request.connectTimeoutSeconds as f64),
    );
    set_property(
        &object,
        "readTimeoutSeconds",
        JsValue::from_f64(request.readTimeoutSeconds as f64),
    );
    set_property(
        &object,
        "followRedirects",
        JsValue::from_bool(request.followRedirects),
    );
    set_property(&object, "ignoreSsl", JsValue::from_bool(request.ignoreSsl));
    set_property(&object, "proxyHost", JsValue::from_str(&request.proxyHost));
    set_property(
        &object,
        "proxyPort",
        JsValue::from_f64(request.proxyPort as f64),
    );
    object.into()
}

pub(crate) fn js_http_response(value: JsValue) -> HostResult<HttpResponseData> {
    Ok(HttpResponseData {
        finalUrl: read_string_property(&value, "finalUrl")?,
        statusCode: read_i32_property(&value, "statusCode")?,
        statusMessage: read_string_property(&value, "statusMessage")?,
        headers: js_string_pairs(
            Reflect::get(&value, &JsValue::from_str("headers")).map_err(js_error)?,
            "http response headers",
        )?,
        body: Uint8Array::new(&Reflect::get(&value, &JsValue::from_str("body")).map_err(js_error)?)
            .to_vec(),
    })
}

pub(crate) fn runtime_process_request_to_js(request: &RuntimeProcessRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "program", program_to_js(request.program.clone()));
    set_property(
        &object,
        "executablePath",
        optional_string_to_js(request.executablePath.as_ref()),
    );
    set_property(&object, "args", strings_to_js(&request.args));
    set_property(&object, "cwd", optional_string_to_js(request.cwd.as_ref()));
    set_property(&object, "env", string_map_to_js(&request.env));
    object.into()
}

pub(crate) fn program_to_js(program: ManagedRuntimeProgram) -> JsValue {
    match program {
        ManagedRuntimeProgram::Node => JsValue::from_str("node"),
        ManagedRuntimeProgram::Python => JsValue::from_str("python"),
        ManagedRuntimeProgram::Uv => JsValue::from_str("uv"),
        ManagedRuntimeProgram::Pnpm => JsValue::from_str("pnpm"),
    }
}

pub(crate) fn system_setting_data(value: JsValue) -> HostResult<SystemSettingData> {
    Ok(SystemSettingData {
        namespace: read_string_property(&value, "namespace")?,
        setting: read_string_property(&value, "setting")?,
        value: read_string_property(&value, "value")?,
    })
}

pub(crate) fn app_operation_data(value: JsValue) -> HostResult<AppOperationData> {
    Ok(AppOperationData {
        operationType: read_string_property(&value, "operationType")?,
        packageName: read_string_property(&value, "packageName")?,
        success: read_bool_property(&value, "success")?,
        details: read_string_property(&value, "details")?,
    })
}

pub(crate) fn sqlite_params_to_js(params: Vec<SqliteValue>) -> JsValue {
    let array = Array::new();
    for param in params {
        array.push(&sqlite_value_to_js(param));
    }
    array.into()
}

fn sqlite_value_to_js(value: SqliteValue) -> JsValue {
    let object = Object::new();
    match value {
        SqliteValue::Null => {
            set_property(&object, "kind", JsValue::from_str("null"));
        }
        SqliteValue::Integer(value) => {
            set_property(&object, "kind", JsValue::from_str("integer"));
            set_property(&object, "value", JsValue::from_str(&value.to_string()));
        }
        SqliteValue::Real(value) => {
            set_property(&object, "kind", JsValue::from_str("real"));
            set_property(&object, "value", JsValue::from_f64(value));
        }
        SqliteValue::Text(value) => {
            set_property(&object, "kind", JsValue::from_str("text"));
            set_property(&object, "value", JsValue::from_str(&value));
        }
        SqliteValue::Blob(value) => {
            set_property(&object, "kind", JsValue::from_str("blob"));
            set_property(&object, "value", bytes_to_js(&value));
        }
    }
    object.into()
}

fn js_to_sqlite_value(value: JsValue) -> HostResult<SqliteValue> {
    let kind = read_string_property(&value, "kind")?;
    let raw_value = Reflect::get(&value, &JsValue::from_str("value")).map_err(js_error)?;
    match kind.as_str() {
        "null" => Ok(SqliteValue::Null),
        "integer" => Ok(SqliteValue::Integer(js_i64(raw_value, "sqlite integer")?)),
        "real" => Ok(SqliteValue::Real(js_f64(raw_value, "sqlite real")?)),
        "text" => Ok(SqliteValue::Text(js_string(raw_value, "sqlite text")?)),
        "blob" => Ok(SqliteValue::Blob(Uint8Array::new(&raw_value).to_vec())),
        other => Err(HostError::new(format!(
            "unknown sqlite value kind: {other}"
        ))),
    }
}

pub(crate) fn js_rows(value: JsValue) -> HostResult<Vec<SqliteRow>> {
    let array = Array::from(&value);
    let mut rows = Vec::new();
    for index in 0..array.length() {
        let row = array.get(index);
        let columns = js_string_array(
            Reflect::get(&row, &JsValue::from_str("columns")).map_err(js_error)?,
            "sqlite row columns",
        )?;
        let values =
            Array::from(&Reflect::get(&row, &JsValue::from_str("values")).map_err(js_error)?);
        let mut row_values = Vec::new();
        for value_index in 0..values.length() {
            row_values.push(js_to_sqlite_value(values.get(value_index))?);
        }
        rows.push(SqliteRow {
            columns,
            values: row_values,
        });
    }
    Ok(rows)
}

pub(crate) fn js_visit_links(value: JsValue) -> HostResult<Vec<WebVisitLinkData>> {
    let array = Array::from(&value);
    let mut links = Vec::new();
    for index in 0..array.length() {
        let link = array.get(index);
        links.push(WebVisitLinkData {
            url: read_string_property(&link, "url")?,
            text: read_string_property(&link, "text")?,
        });
    }
    Ok(links)
}

pub(crate) fn js_string_pairs(value: JsValue, context: &str) -> HostResult<Vec<(String, String)>> {
    let array = Array::from(&value);
    let mut pairs = Vec::new();
    for index in 0..array.length() {
        let pair = array.get(index);
        if Array::is_array(&pair) {
            let pair_array = Array::from(&pair);
            pairs.push((
                js_string(pair_array.get(0), context)?,
                js_string(pair_array.get(1), context)?,
            ));
        } else {
            pairs.push((
                read_string_property(&pair, "key")?,
                read_string_property(&pair, "value")?,
            ));
        }
    }
    Ok(pairs)
}

pub(crate) fn js_string_array(value: JsValue, context: &str) -> HostResult<Vec<String>> {
    let array = Array::from(&value);
    let mut values = Vec::new();
    for index in 0..array.length() {
        values.push(js_string(array.get(index), context)?);
    }
    Ok(values)
}

pub(crate) fn js_string_map(value: JsValue, context: &str) -> HostResult<BTreeMap<String, String>> {
    let object = value
        .dyn_into::<Object>()
        .map_err(|_| HostError::new(format!("{context} returned non-object value")))?;
    let keys = Object::keys(&object);
    let mut values = BTreeMap::new();
    for index in 0..keys.length() {
        let key = js_string(keys.get(index), context)?;
        let value = Reflect::get(object.as_ref(), &JsValue::from_str(&key)).map_err(js_error)?;
        values.insert(key, js_string(value, context)?);
    }
    Ok(values)
}

pub(crate) fn string_pairs_to_js(values: &[(String, String)]) -> JsValue {
    let array = Array::new();
    for (key, value) in values {
        let pair = Array::new();
        pair.push(&JsValue::from_str(key));
        pair.push(&JsValue::from_str(value));
        array.push(&pair);
    }
    array.into()
}

fn http_file_parts_to_js(values: &[HttpFilePart]) -> JsValue {
    let array = Array::new();
    for value in values {
        let object = Object::new();
        set_property(&object, "fieldName", JsValue::from_str(&value.fieldName));
        set_property(&object, "fileName", JsValue::from_str(&value.fileName));
        set_property(
            &object,
            "contentType",
            JsValue::from_str(&value.contentType),
        );
        set_property(&object, "content", bytes_to_js(&value.content));
        array.push(&object);
    }
    array.into()
}

fn strings_to_js(values: &[String]) -> JsValue {
    let array = Array::new();
    for value in values {
        array.push(&JsValue::from_str(value));
    }
    array.into()
}

fn string_map_to_js(values: &BTreeMap<String, String>) -> JsValue {
    let object = Object::new();
    for (key, value) in values {
        set_property(&object, key, JsValue::from_str(value));
    }
    object.into()
}

fn optional_string_to_js(value: Option<&String>) -> JsValue {
    match value {
        Some(value) => JsValue::from_str(value),
        None => JsValue::NULL,
    }
}

pub(crate) fn optional_str_to_js(value: Option<&str>) -> JsValue {
    match value {
        Some(value) => JsValue::from_str(value),
        None => JsValue::NULL,
    }
}

pub(crate) fn bytes_to_js(content: &[u8]) -> JsValue {
    let bytes = Uint8Array::new_with_length(content.len() as u32);
    bytes.copy_from(content);
    bytes.into()
}

pub(crate) fn read_string_property(value: &JsValue, property: &str) -> HostResult<String> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_string(property_value, property)
}

pub(crate) fn read_optional_string_property(
    value: &JsValue,
    property: &str,
) -> HostResult<Option<String>> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_optional_string(property_value, property)
}

pub(crate) fn read_bool_property(value: &JsValue, property: &str) -> HostResult<bool> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_bool(property_value, property)
}

pub(crate) fn read_i64_property(value: &JsValue, property: &str) -> HostResult<i64> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_i64(property_value, property)
}

pub(crate) fn read_optional_i64_property(
    value: &JsValue,
    property: &str,
) -> HostResult<Option<i64>> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    if property_value.is_null() || property_value.is_undefined() {
        return Ok(None);
    }
    Ok(Some(js_i64(property_value, property)?))
}

pub(crate) fn read_i32_property(value: &JsValue, property: &str) -> HostResult<i32> {
    let value = read_i64_property(value, property)?;
    i32::try_from(value).map_err(|error| HostError::new(error.to_string()))
}

pub(crate) fn read_optional_i32_property(
    value: &JsValue,
    property: &str,
) -> HostResult<Option<i32>> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    if property_value.is_null() || property_value.is_undefined() {
        return Ok(None);
    }
    let value = js_i64(property_value, property)?;
    Ok(Some(
        i32::try_from(value).map_err(|error| HostError::new(error.to_string()))?,
    ))
}

pub(crate) fn read_usize_property(value: &JsValue, property: &str) -> HostResult<usize> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_usize(property_value, property)
}

pub(crate) fn read_f64_property(value: &JsValue, property: &str) -> HostResult<f64> {
    let property_value = Reflect::get(value, &JsValue::from_str(property)).map_err(js_error)?;
    js_f64(property_value, property)
}

pub(crate) fn read_f32_property(value: &JsValue, property: &str) -> HostResult<f32> {
    Ok(read_f64_property(value, property)? as f32)
}

pub(crate) fn js_string(value: JsValue, context: &str) -> HostResult<String> {
    value
        .as_string()
        .ok_or_else(|| HostError::new(format!("{context} returned non-string value")))
}

pub(crate) fn js_optional_string(value: JsValue, context: &str) -> HostResult<Option<String>> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    Ok(Some(js_string(value, context)?))
}

pub(crate) fn js_bool(value: JsValue, context: &str) -> HostResult<bool> {
    value
        .as_bool()
        .ok_or_else(|| HostError::new(format!("{context} returned non-boolean value")))
}

pub(crate) fn js_usize(value: JsValue, context: &str) -> HostResult<usize> {
    let value = js_i64(value, context)?;
    usize::try_from(value).map_err(|error| HostError::new(error.to_string()))
}

pub(crate) fn js_i64(value: JsValue, context: &str) -> HostResult<i64> {
    if let Some(text) = value.as_string() {
        return text.parse::<i64>().map_err(|error| {
            HostError::new(format!("{context} returned invalid integer: {error}"))
        });
    }
    if let Some(number) = value.as_f64() {
        return Ok(number as i64);
    }
    Err(HostError::new(format!(
        "{context} returned non-integer value"
    )))
}

pub(crate) fn js_f64(value: JsValue, context: &str) -> HostResult<f64> {
    value
        .as_f64()
        .ok_or_else(|| HostError::new(format!("{context} returned non-number value")))
}

pub(crate) fn set_property(object: &Object, property: &str, value: JsValue) {
    Reflect::set(object.as_ref(), &JsValue::from_str(property), &value)
        .expect("setting property on a fresh JS object must succeed");
}

pub(crate) fn js_error(value: JsValue) -> HostError {
    if let Some(message) = value.as_string() {
        HostError::new(message)
    } else {
        HostError::new(format!("{value:?}"))
    }
}

pub(crate) fn bluetooth_state_data(value: JsValue) -> HostResult<BluetoothStateData> {
    Ok(BluetoothStateData {
        supported: read_bool_property(&value, "supported")?,
        enabled: read_bool_property(&value, "enabled")?,
        state: read_string_property(&value, "state")?,
    })
}

pub(crate) fn bluetooth_device_data(value: JsValue) -> HostResult<BluetoothDeviceData> {
    Ok(BluetoothDeviceData {
        name: read_optional_string_property(&value, "name")?,
        address: read_string_property(&value, "address")?,
        r#type: read_string_property(&value, "type")?,
        bondState: read_string_property(&value, "bondState")?,
    })
}

pub(crate) fn bluetooth_bonded_devices_data(
    value: JsValue,
) -> HostResult<BluetoothBondedDevicesData> {
    let array =
        Array::from(&Reflect::get(&value, &JsValue::from_str("devices")).map_err(js_error)?);
    let mut devices = Vec::new();
    for index in 0..array.length() {
        devices.push(bluetooth_device_data(array.get(index))?);
    }
    Ok(BluetoothBondedDevicesData { devices })
}

pub(crate) fn bluetooth_scanned_device_data(
    value: JsValue,
) -> HostResult<BluetoothScannedDeviceData> {
    Ok(BluetoothScannedDeviceData {
        name: read_optional_string_property(&value, "name")?,
        address: read_string_property(&value, "address")?,
        r#type: read_string_property(&value, "type")?,
        bondState: read_string_property(&value, "bondState")?,
        source: read_string_property(&value, "source")?,
        rssi: read_optional_i32_property(&value, "rssi")?,
    })
}

pub(crate) fn bluetooth_scan_result_data(value: JsValue) -> HostResult<BluetoothScanResultData> {
    let array =
        Array::from(&Reflect::get(&value, &JsValue::from_str("devices")).map_err(js_error)?);
    let mut devices = Vec::new();
    for index in 0..array.length() {
        devices.push(bluetooth_scanned_device_data(array.get(index))?);
    }
    Ok(BluetoothScanResultData {
        devices,
        durationMs: read_i64_property(&value, "durationMs")?,
        includesBle: read_bool_property(&value, "includesBle")?,
    })
}

pub(crate) fn bluetooth_session_data(value: JsValue) -> HostResult<BluetoothSessionData> {
    Ok(BluetoothSessionData {
        sessionId: read_string_property(&value, "sessionId")?,
        address: read_string_property(&value, "address")?,
        mode: read_string_property(&value, "mode")?,
    })
}

pub(crate) fn bluetooth_transfer_data(value: JsValue) -> HostResult<BluetoothTransferData> {
    Ok(BluetoothTransferData {
        sessionId: read_string_property(&value, "sessionId")?,
        bytesWritten: read_i64_property(&value, "bytesWritten")?,
    })
}

pub(crate) fn bluetooth_read_data(value: JsValue) -> HostResult<BluetoothReadData> {
    Ok(BluetoothReadData {
        sessionId: read_string_property(&value, "sessionId")?,
        bytesRead: read_i64_property(&value, "bytesRead")?,
        text: read_optional_string_property(&value, "text")?,
        dataBase64: read_optional_string_property(&value, "dataBase64")?,
    })
}

pub(crate) fn bluetooth_ble_services_data(value: JsValue) -> HostResult<BluetoothBleServicesData> {
    let services_value = Reflect::get(&value, &JsValue::from_str("services")).map_err(js_error)?;
    let services_array = Array::from(&services_value);
    let mut services = Vec::new();
    for service_index in 0..services_array.length() {
        let service = services_array.get(service_index);
        let characteristics_value =
            Reflect::get(&service, &JsValue::from_str("characteristics")).map_err(js_error)?;
        let characteristics_array = Array::from(&characteristics_value);
        let mut characteristics = Vec::new();
        for characteristic_index in 0..characteristics_array.length() {
            let characteristic = characteristics_array.get(characteristic_index);
            characteristics.push(BluetoothBleCharacteristicData {
                uuid: read_string_property(&characteristic, "uuid")?,
                properties: js_string_array(
                    Reflect::get(&characteristic, &JsValue::from_str("properties"))
                        .map_err(js_error)?,
                    "bluetooth characteristic properties",
                )?,
            });
        }
        services.push(BluetoothBleServiceData {
            uuid: read_string_property(&service, "uuid")?,
            characteristics,
        });
    }
    Ok(BluetoothBleServicesData {
        sessionId: read_string_property(&value, "sessionId")?,
        services,
    })
}

pub(crate) fn bluetooth_ble_notification_data(
    value: JsValue,
) -> HostResult<BluetoothBleNotificationData> {
    let notifications_value =
        Reflect::get(&value, &JsValue::from_str("notifications")).map_err(js_error)?;
    let notifications_array = Array::from(&notifications_value);
    let mut notifications = Vec::new();
    for index in 0..notifications_array.length() {
        let notification = notifications_array.get(index);
        notifications.push(BluetoothBleNotificationEntry {
            characteristicUuid: read_string_property(&notification, "characteristicUuid")?,
            bytesRead: read_i64_property(&notification, "bytesRead")?,
            text: read_optional_string_property(&notification, "text")?,
            dataBase64: read_optional_string_property(&notification, "dataBase64")?,
            timestamp: read_i64_property(&notification, "timestamp")?,
        });
    }
    Ok(BluetoothBleNotificationData {
        sessionId: read_string_property(&value, "sessionId")?,
        notifications,
    })
}
