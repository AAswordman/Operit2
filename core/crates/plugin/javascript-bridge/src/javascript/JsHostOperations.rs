use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::{Mutex, OnceLock};

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256};
use base64::Engine;
use flate2::read::DeflateDecoder;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Rgba};
use md5::{Digest, Md5};
use operit_plugin_sdk::javascript::{JsToolCallRequest, JsToolCallResult, JsToolCallResultData};

const BINARY_HANDLE_PREFIX: &str = "@binary_handle:";
const BINARY_DATA_THRESHOLD: usize = 32 * 1024;

#[allow(non_snake_case)]
struct SerializedToolResultData {
    data: serde_json::Value,
    dataType: Option<&'static str>,
}

/// Consumes structured parameters instead of cloning the parsed object tree.
#[allow(non_snake_case)]
pub fn parseToolCallValue(
    toolType: &str,
    toolName: &str,
    value: serde_json::Value,
) -> Result<JsToolCallRequest, String> {
    let normalizedToolName = toolName.trim();
    if normalizedToolName.is_empty() {
        return Err("Tool name cannot be empty".to_string());
    }
    let serde_json::Value::Object(object) = value else {
        return Err("Tool params must be a JSON object".to_string());
    };
    Ok(JsToolCallRequest {
        tool_type: toolType.trim().to_string(),
        tool_name: normalizedToolName.to_string(),
        parameters: object.into_iter().collect(),
    })
}

static BINARY_DATA_REGISTRY: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
static BITMAP_REGISTRY: OnceLock<Mutex<HashMap<String, DynamicImage>>> = OnceLock::new();

/// Returns the shared registry of explicitly owned binary result handles.
fn binaryDataRegistry() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    BINARY_DATA_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns the shared registry of runtime-owned image handles.
fn bitmapRegistry() -> &'static Mutex<HashMap<String, DynamicImage>> {
    BITMAP_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Reads bytes according to the explicit binary-handle or base64 data contract.
#[allow(non_snake_case)]
fn readBinaryOrBase64(data: &str) -> Result<Vec<u8>, String> {
    if let Some(handle) = data.strip_prefix(BINARY_HANDLE_PREFIX) {
        let mut guard = binaryDataRegistry()
            .lock()
            .expect("binary data registry mutex poisoned");
        return guard
            .remove(handle)
            .ok_or_else(|| format!("Invalid or expired binary handle: {handle}"));
    }
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| error.to_string())
}

/// Decompresses explicit deflate data and propagates decoding failures.
#[allow(non_snake_case)]
pub fn decompress(data: &str, algorithm: &str) -> Result<String, String> {
    if algorithm.to_ascii_lowercase() != "deflate" {
        return Err(format!(
            "Unsupported algorithm: {algorithm}. Only 'deflate' is supported."
        ));
    }
    let compressedData = readBinaryOrBase64(data)?;
    if compressedData.is_empty() {
        return Ok(String::new());
    }
    let mut decoder = DeflateDecoder::new(compressedData.as_slice());
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|error| error.to_string())?;
    String::from_utf8(output).map_err(|error| error.to_string())
}

/// Executes one crypto operation using an already-structured argument array.
#[allow(non_snake_case)]
pub fn crypto(algorithm: &str, operation: &str, args: &[String]) -> Result<String, String> {
    match algorithm.to_ascii_lowercase().as_str() {
        "md5" => {
            let [input] = args else {
                return Err("MD5 hash requires exactly one input string".to_string());
            };
            if !operation.eq_ignore_ascii_case("hash") {
                return Err(format!("Unknown MD5 operation: {operation}"));
            }
            let mut hasher = Md5::new();
            hasher.update(input.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        }
        "aes" => match operation.to_ascii_lowercase().as_str() {
            "decrypt" => {
                let [data, key] = args else {
                    return Err("AES decryption requires data and key strings".to_string());
                };
                decryptAesEcbNoPaddingPkcs7(data, key)
            }
            _ => Err(format!("Unknown AES operation: {operation}")),
        },
        _ => Err(format!("Unknown algorithm: {algorithm}")),
    }
}

/// Decrypts AES ECB bytes and validates the declared key and padding.
#[allow(non_snake_case)]
fn decryptAesEcbNoPaddingPkcs7(data: &str, key: &str) -> Result<String, String> {
    let mut decodedData = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| error.to_string())?;
    if decodedData.len() % 16 != 0 {
        return Err(
            "Input length must be multiple of 16 when decrypting with padded cipher".to_string(),
        );
    }
    let keyBytes = key.as_bytes();
    match keyBytes.len() {
        16 => decryptAesBlocks::<Aes128>(&mut decodedData, keyBytes)?,
        24 => decryptAesBlocks::<Aes192>(&mut decodedData, keyBytes)?,
        32 => decryptAesBlocks::<Aes256>(&mut decodedData, keyBytes)?,
        _ => return Err("Invalid AES key length".to_string()),
    }
    if decodedData.is_empty() {
        return Ok(String::new());
    }
    let paddingLength = *decodedData
        .last()
        .ok_or_else(|| "Invalid PKCS7 padding length: 0".to_string())?
        as usize;
    if paddingLength < 1 || paddingLength > decodedData.len() {
        return Err(format!("Invalid PKCS7 padding length: {paddingLength}"));
    }
    decodedData.truncate(decodedData.len() - paddingLength);
    String::from_utf8(decodedData).map_err(|error| error.to_string())
}

/// Decrypts each complete AES block using the selected cipher type.
#[allow(non_snake_case)]
fn decryptAesBlocks<C>(data: &mut [u8], key: &[u8]) -> Result<(), String>
where
    C: BlockDecrypt + KeyInit,
{
    let cipher = C::new_from_slice(key).map_err(|error| error.to_string())?;
    for block in data.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
    }
    Ok(())
}

/// Executes one image operation using owned structured argument values.
#[allow(non_snake_case)]
pub fn imageProcessing(
    operation: &str,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    match operation.to_ascii_lowercase().as_str() {
        "read" => {
            let data = args
                .get(0)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Image data is required".to_string())?;
            let decodedBytes = readBinaryOrBase64(data)?;
            let image =
                image::load_from_memory(&decodedBytes).map_err(|error| error.to_string())?;
            let id = uuid::Uuid::new_v4().to_string();
            bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned")
                .insert(id.clone(), image);
            Ok(serde_json::Value::String(id))
        }
        "create" => {
            let width = jsonIntArg(args, 0)?;
            let height = jsonIntArg(args, 1)?;
            let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                width,
                height,
                Rgba([0, 0, 0, 0]),
            ));
            let id = uuid::Uuid::new_v4().to_string();
            bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned")
                .insert(id.clone(), image);
            Ok(serde_json::Value::String(id))
        }
        "crop" => {
            let id = jsonStringArg(args, 0)?;
            let x = jsonIntArg(args, 1)?;
            let y = jsonIntArg(args, 2)?;
            let width = jsonIntArg(args, 3)?;
            let height = jsonIntArg(args, 4)?;
            let cropped = {
                let guard = bitmapRegistry()
                    .lock()
                    .expect("bitmap registry mutex poisoned");
                let image = guard
                    .get(&id)
                    .ok_or_else(|| format!("Source bitmap not found for crop (ID: {id})"))?;
                image.crop_imm(x, y, width, height)
            };
            let newId = uuid::Uuid::new_v4().to_string();
            bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned")
                .insert(newId.clone(), cropped);
            Ok(serde_json::Value::String(newId))
        }
        "composite" => {
            let baseId = jsonStringArg(args, 0)?;
            let srcId = jsonStringArg(args, 1)?;
            let x = jsonIntArg(args, 2)? as i64;
            let y = jsonIntArg(args, 3)? as i64;
            let mut guard = bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned");
            let srcImage = guard
                .get(&srcId)
                .ok_or_else(|| format!("Source bitmap not found for composite (ID: {srcId})"))?
                .clone();
            let baseImage = guard
                .get_mut(&baseId)
                .ok_or_else(|| format!("Base bitmap not found for composite (ID: {baseId})"))?;
            image::imageops::overlay(baseImage, &srcImage, x, y);
            Ok(serde_json::Value::Null)
        }
        "getwidth" => {
            let id = jsonStringArg(args, 0)?;
            let guard = bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned");
            let width = guard
                .get(&id)
                .ok_or_else(|| format!("Bitmap not found for getWidth (ID: {id})"))?
                .width();
            Ok(serde_json::Value::Number(serde_json::Number::from(width)))
        }
        "getheight" => {
            let id = jsonStringArg(args, 0)?;
            let guard = bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned");
            let height = guard
                .get(&id)
                .ok_or_else(|| format!("Bitmap not found for getHeight (ID: {id})"))?
                .height();
            Ok(serde_json::Value::Number(serde_json::Number::from(height)))
        }
        "getbase64" => {
            let id = jsonStringArg(args, 0)?;
            let mime = jsonStringArg(args, 1)?;
            if mime != "image/png" && mime != "image/jpeg" {
                return Err(format!("Unsupported image output MIME type: {mime}"));
            }
            let guard = bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned");
            let image = guard
                .get(&id)
                .ok_or_else(|| format!("Bitmap not found for getBase64 (ID: {id})"))?;
            let mut bytes = Vec::new();
            if mime == "image/png" {
                image
                    .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
                    .map_err(|error| error.to_string())?;
            } else {
                let rgb = image.to_rgb8();
                let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
                encoder
                    .encode_image(&DynamicImage::ImageRgb8(rgb))
                    .map_err(|error| error.to_string())?;
            }
            Ok(serde_json::Value::String(
                base64::engine::general_purpose::STANDARD.encode(bytes),
            ))
        }
        "release" => {
            let id = jsonStringArg(args, 0)?;
            bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned")
                .remove(&id);
            Ok(serde_json::Value::Null)
        }
        _ => Err(format!("Unknown image operation: {operation}")),
    }
}

/// Requires a string at the exact image-operation argument position.
#[allow(non_snake_case)]
fn jsonStringArg(args: &[serde_json::Value], index: usize) -> Result<String, String> {
    args.get(index)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Argument {index} must be a string"))
}

/// Requires a non-negative integer at the exact image-operation argument position.
#[allow(non_snake_case)]
fn jsonIntArg(args: &[serde_json::Value], index: usize) -> Result<u32, String> {
    let value = args
        .get(index)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("Argument {index} must be an integer"))?;
    u32::try_from(value).map_err(|_| format!("Argument {index} must be an integer"))
}

/// Builds the existing tool envelope without producing JSON text or cloning data.
#[allow(non_snake_case)]
pub fn toolExecutionResultValue(result: JsToolCallResult) -> serde_json::Value {
    let serializedData = serializeToolResultData(result.data);
    let mut object = serde_json::Map::new();
    object.insert(
        "success".to_string(),
        serde_json::Value::Bool(result.success),
    );
    if !result.success {
        object.insert(
            "message".to_string(),
            serde_json::Value::String(result.error.unwrap_or_default()),
        );
    }
    object.insert("data".to_string(), serializedData.data);
    if let Some(dataType) = serializedData.dataType {
        object.insert(
            "dataType".to_string(),
            serde_json::Value::String(dataType.to_string()),
        );
    }
    serde_json::Value::Object(object)
}

/// Encodes generic SDK result data for JavaScript consumption.
#[allow(non_snake_case)]
fn serializeToolResultData(result: JsToolCallResultData) -> SerializedToolResultData {
    match result {
        JsToolCallResultData::Binary(data) => {
            let encodedData = if data.len() > BINARY_DATA_THRESHOLD {
                let handle = uuid::Uuid::new_v4().to_string();
                binaryDataRegistry()
                    .lock()
                    .expect("binary data registry mutex poisoned")
                    .insert(handle.clone(), data);
                format!("{BINARY_HANDLE_PREFIX}{handle}")
            } else {
                base64::engine::general_purpose::STANDARD.encode(&data)
            };
            SerializedToolResultData {
                data: serde_json::Value::String(encodedData),
                dataType: Some("base64"),
            }
        }
        JsToolCallResultData::Value(data) => SerializedToolResultData {
            data,
            dataType: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{toolExecutionResultValue, BINARY_DATA_THRESHOLD, BINARY_HANDLE_PREFIX};
    use operit_plugin_sdk::javascript::{JsToolCallResult, JsToolCallResultData};
    use serde_json::Value;

    /// Builds a successful SDK tool result for serialization tests.
    fn successResult(data: JsToolCallResultData) -> JsToolCallResult {
        JsToolCallResult {
            success: true,
            data,
            error: None,
        }
    }

    /// Preserves application strings without interpreting their JSON-looking contents.
    #[test]
    fn string_result_data_stays_literal_string_for_js_tool_result() {
        let payload = r#"{"__type":"PackageOwnedType","value":"plain package json"}"#;
        let result = successResult(JsToolCallResultData::Value(Value::String(
            payload.to_string(),
        )));

        let serialized = toolExecutionResultValue(result);

        assert_eq!(serialized["success"], Value::Bool(true));
        assert_eq!(serialized["data"], Value::String(payload.to_string()));
        assert!(serialized.get("dataType").is_none());
    }

    /// Keeps structured object fields in the direct tool result envelope.
    #[test]
    fn structured_result_data_serializes_json_object_for_js_tool_result() {
        let result = successResult(JsToolCallResultData::Value(serde_json::json!({
            "__type": "TerminalCommandResultData",
            "command": "Write-Output ok",
            "output": "ok\n",
            "exitCode": 0,
            "sessionId": "session-1",
            "terminalType": "powershell",
            "timedOut": false
        })));

        let serialized = toolExecutionResultValue(result);

        assert_eq!(serialized["success"], Value::Bool(true));
        assert_eq!(serialized["data"]["__type"], "TerminalCommandResultData");
        assert_eq!(serialized["data"]["command"], "Write-Output ok");
        assert!(serialized.get("dataType").is_none());
    }

    /// Preserves the explicit base64 metadata of small binary results.
    #[test]
    fn binary_result_data_serializes_base64_metadata_for_js_tool_result() {
        let result = successResult(JsToolCallResultData::Binary(b"hello".to_vec()));

        let serialized = toolExecutionResultValue(result);

        assert_eq!(serialized["data"], "aGVsbG8=");
        assert_eq!(serialized["dataType"], "base64");
    }

    /// Transfers large binary results through the explicit owned handle contract.
    #[test]
    fn large_binary_result_data_serializes_handle_for_js_tool_result() {
        let result = successResult(JsToolCallResultData::Binary(vec![
            7;
            BINARY_DATA_THRESHOLD + 1
        ]));

        let serialized = toolExecutionResultValue(result);
        let data = serialized["data"].as_str().expect("binary handle");

        assert!(data.starts_with(BINARY_HANDLE_PREFIX));
        assert_eq!(serialized["dataType"], "base64");
    }
}
