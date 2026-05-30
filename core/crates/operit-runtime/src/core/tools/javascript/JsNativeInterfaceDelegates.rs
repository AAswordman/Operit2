use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::sync::{Mutex, OnceLock};

use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use crate::core::tools::AIToolHandler::AIToolHandler;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256};
use base64::Engine;
use flate2::read::DeflateDecoder;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Rgba};
use md5::{Digest, Md5};

const BINARY_HANDLE_PREFIX: &str = "@binary_handle:";

#[derive(Clone, Debug)]
struct ParsedToolCall {
    params: BTreeMap<String, String>,
    fullToolName: String,
    aiTool: AITool,
}

#[allow(non_snake_case)]
fn buildToolErrorJson(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message
    })
    .to_string()
}

#[allow(non_snake_case)]
fn parseToolCall(
    toolType: &str,
    toolName: &str,
    paramsJson: &str,
) -> Result<ParsedToolCall, String> {
    let normalizedToolName = toolName.trim();
    if normalizedToolName.is_empty() {
        return Err("Tool name cannot be empty".to_string());
    }

    let value =
        serde_json::from_str::<serde_json::Value>(paramsJson).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "Tool params must be a JSON object".to_string())?;

    let mut params = BTreeMap::new();
    for (key, value) in object {
        let text = match value {
            serde_json::Value::Null => String::new(),
            serde_json::Value::String(value) => value.clone(),
            _ => value.to_string(),
        };
        params.insert(key.clone(), text);
    }

    let fullToolName = if !toolType.is_empty() && toolType != "default" {
        format!("{toolType}:{normalizedToolName}")
    } else {
        normalizedToolName.to_string()
    };
    let toolParameters = params
        .iter()
        .map(|(name, value)| ToolParameter {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();

    Ok(ParsedToolCall {
        params,
        fullToolName: fullToolName.clone(),
        aiTool: AITool {
            name: fullToolName,
            parameters: toolParameters,
        },
    })
}

static BINARY_DATA_REGISTRY: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
static BITMAP_REGISTRY: OnceLock<Mutex<HashMap<String, DynamicImage>>> = OnceLock::new();

fn binaryDataRegistry() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    BINARY_DATA_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bitmapRegistry() -> &'static Mutex<HashMap<String, DynamicImage>> {
    BITMAP_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[allow(non_snake_case)]
fn nativeErrorJson(message: String) -> String {
    serde_json::json!({
        "nativeError": message.replace('"', "'")
    })
    .to_string()
}

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

#[allow(non_snake_case)]
pub fn decompress(data: &str, algorithm: &str) -> String {
    let result = (|| -> Result<String, String> {
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
    })();
    match result {
        Ok(value) => value,
        Err(error) => nativeErrorJson(error),
    }
}

#[allow(non_snake_case)]
pub fn crypto(algorithm: &str, operation: &str, argsJson: &str) -> String {
    let result = (|| -> Result<String, String> {
        let args =
            serde_json::from_str::<Vec<String>>(argsJson).map_err(|error| error.to_string())?;
        match algorithm.to_ascii_lowercase().as_str() {
            "md5" => {
                let input = args.get(0).cloned().unwrap_or_default();
                let mut hasher = Md5::new();
                hasher.update(input.as_bytes());
                Ok(format!("{:x}", hasher.finalize()))
            }
            "aes" => match operation.to_ascii_lowercase().as_str() {
                "decrypt" => {
                    let data = args.get(0).cloned().unwrap_or_default();
                    let key = args
                        .get(1)
                        .ok_or_else(|| "Missing key for AES decryption".to_string())?;
                    decryptAesEcbNoPaddingPkcs7(&data, key)
                }
                _ => Err(format!("Unknown AES operation: {operation}")),
            },
            _ => Err(format!("Unknown algorithm: {algorithm}")),
        }
    })();
    match result {
        Ok(value) => value,
        Err(error) => nativeErrorJson(error),
    }
}

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

#[allow(non_snake_case)]
pub fn imageProcessing(operation: &str, argsJson: &str) -> Result<serde_json::Value, String> {
    let args = serde_json::from_str::<Vec<serde_json::Value>>(argsJson)
        .map_err(|error| error.to_string())?;
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
            let width = jsonIntArg(&args, 0)?;
            let height = jsonIntArg(&args, 1)?;
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
            let id = jsonStringArg(&args, 0)?;
            let x = jsonIntArg(&args, 1)?;
            let y = jsonIntArg(&args, 2)?;
            let width = jsonIntArg(&args, 3)?;
            let height = jsonIntArg(&args, 4)?;
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
            let baseId = jsonStringArg(&args, 0)?;
            let srcId = jsonStringArg(&args, 1)?;
            let x = jsonIntArg(&args, 2)? as i64;
            let y = jsonIntArg(&args, 3)? as i64;
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
            let id = jsonStringArg(&args, 0)?;
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
            let id = jsonStringArg(&args, 0)?;
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
            let id = jsonStringArg(&args, 0)?;
            let mime = args
                .get(1)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("image/jpeg");
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
            let id = jsonStringArg(&args, 0)?;
            bitmapRegistry()
                .lock()
                .expect("bitmap registry mutex poisoned")
                .remove(&id);
            Ok(serde_json::Value::Null)
        }
        _ => Err(format!("Unknown image operation: {operation}")),
    }
}

#[allow(non_snake_case)]
fn jsonStringArg(args: &[serde_json::Value], index: usize) -> Result<String, String> {
    args.get(index)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Argument {index} must be a string"))
}

#[allow(non_snake_case)]
fn jsonIntArg(args: &[serde_json::Value], index: usize) -> Result<u32, String> {
    let value = args
        .get(index)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("Argument {index} must be an integer"))?;
    u32::try_from(value).map_err(|_| format!("Argument {index} must be an integer"))
}

#[allow(non_snake_case)]
fn serializeToolExecutionResult(
    result: &crate::api::chat::enhance::ConversationMarkupManager::ToolResult,
) -> String {
    let mut object = serde_json::Map::new();
    object.insert(
        "success".to_string(),
        serde_json::Value::Bool(result.success),
    );
    if !result.success {
        object.insert(
            "message".to_string(),
            serde_json::Value::String(result.error.clone().unwrap_or_default()),
        );
    }
    object.insert(
        "data".to_string(),
        serde_json::Value::String(result.result.clone()),
    );
    serde_json::Value::Object(object).to_string()
}

#[allow(non_snake_case)]
pub fn callToolSync(
    toolHandler: &AIToolHandler,
    toolType: &str,
    toolName: &str,
    paramsJson: &str,
) -> String {
    if toolName.trim().is_empty() {
        return buildToolErrorJson("Tool name cannot be empty");
    }

    let parsed = match parseToolCall(toolType, toolName, paramsJson) {
        Ok(value) => value,
        Err(error) => return buildToolErrorJson(&error),
    };
    let _ = parsed.params.len();
    let _ = parsed.fullToolName.as_str();

    let mut handler = toolHandler.clone();
    let result = handler.executeTool(parsed.aiTool);
    serializeToolExecutionResult(&result)
}
