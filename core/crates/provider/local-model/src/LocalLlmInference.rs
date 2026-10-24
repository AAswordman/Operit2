use crate::LocalInference::{LocalChatResponse, LocalModelSelection};
use crate::LocalModelManifest::{LocalModelDriver, LocalModelKind};
use crate::LocalModelProvider::LocalModelProvider;
use serde_json::Value;

/// Resolves exact installed weights and executes a native text chat request.
pub fn generateLocalChat(
    provider: &LocalModelProvider,
    selection: &LocalModelSelection,
    body: Value,
    settings: Value,
) -> Result<LocalChatResponse, String> {
    validateChatRequest(&body, &settings)?;
    let runtime = provider
        .resolve(selection, LocalModelKind::Chat)
        .map_err(|error| error.to_string())?;
    let (engine, relative) = match runtime.model.manifest.driver.as_ref() {
        Some(LocalModelDriver::LlamaCpp { model }) => ("llama-cpp", model),
        Some(LocalModelDriver::Mnn { config }) => ("mnn", config),
        _ => return Err("Local model has no supported chat driver".into()),
    };
    let relativePath = std::path::Path::new(relative);
    if relativePath.is_absolute()
        || relativePath
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("Local model entry path escapes its installation directory".into());
    }
    let path = std::path::Path::new(&runtime.modelDirectory).join(relativePath);
    if !path.is_file() {
        return Err(format!("Local model file is missing: {}", path.display()));
    }
    nativeGenerate(engine, &path.to_string_lossy(), &body, &settings)
}

fn validateChatRequest(body: &Value, settings: &Value) -> Result<(), String> {
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or("Local chat requires a non-empty messages array")?;
    for message in messages {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .ok_or("Local chat message has no role")?;
        if !["system", "user", "assistant", "tool"].contains(&role) {
            return Err(format!("Unsupported local chat role: {role}"));
        }
        if !message
            .get("content")
            .is_some_and(|value| value.is_string() || value.is_array() || value.is_null())
        {
            return Err("Local chat message has invalid content".into());
        }
    }
    let maximum = body
        .get("max_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(512);
    if !(1..=131_072).contains(&maximum) {
        return Err("max_tokens must be between 1 and 131072".into());
    }
    for (name, minimum, maximum) in [
        ("llamaThreadCount", 1, 256),
        ("mnnThreadCount", 1, 256),
        ("llamaContextSize", 128, 1_048_576),
        ("llamaBatchSize", 1, 65_536),
        ("llamaUBatchSize", 1, 65_536),
    ] {
        if let Some(value) = settings.get(name) {
            if !value
                .as_i64()
                .is_some_and(|value| value >= minimum && value <= maximum)
            {
                return Err(format!("Invalid local inference setting: {name}"));
            }
        }
    }
    if settings
        .get("llamaGpuLayers")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        != 0
        || settings
            .get("mnnForwardType")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            != 0
    {
        return Err("This embedded local inference build supports the CPU backend".into());
    }
    Ok(())
}

#[cfg(all(feature = "native-llm", not(target_arch = "wasm32")))]
fn nativeGenerate(
    engine: &str,
    path: &str,
    body: &Value,
    settings: &Value,
) -> Result<LocalChatResponse, String> {
    use std::ffi::{c_char, CStr, CString};
    extern "C" {
        fn operit_local_llm_generate(
            engine: *const c_char,
            path: *const c_char,
            request: *const c_char,
            settings: *const c_char,
        ) -> *mut c_char;
        fn operit_local_llm_free(text: *mut c_char);
    }
    let engine = CString::new(engine).map_err(|error| error.to_string())?;
    let path = CString::new(path).map_err(|error| error.to_string())?;
    let body = CString::new(body.to_string()).map_err(|error| error.to_string())?;
    let settings = CString::new(settings.to_string()).map_err(|error| error.to_string())?;
    // Native adapter owns the allocation. Copy it before freeing with the same allocator.
    let bytes = unsafe {
        let output = operit_local_llm_generate(
            engine.as_ptr(),
            path.as_ptr(),
            body.as_ptr(),
            settings.as_ptr(),
        );
        if output.is_null() {
            return Err("Native local inference ran out of memory".into());
        }
        let bytes = CStr::from_ptr(output).to_bytes().to_vec();
        operit_local_llm_free(output);
        bytes
    };
    let result: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if let Some(error) = result.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }
    serde_json::from_value(result).map_err(|error| error.to_string())
}

#[cfg(not(all(feature = "native-llm", not(target_arch = "wasm32"))))]
fn nativeGenerate(_: &str, _: &str, _: &Value, _: &Value) -> Result<LocalChatResponse, String> {
    Err("Native local LLM inference is unavailable in this build".into())
}

/// Releases the currently cached model weights on this device.
pub fn unloadLocalChat() {
    #[cfg(all(feature = "native-llm", not(target_arch = "wasm32")))]
    unsafe {
        extern "C" {
            fn operit_local_llm_unload();
        }
        operit_local_llm_unload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[cfg(all(feature = "native-llm", not(target_arch = "wasm32")))]
    #[test]
    #[ignore = "requires OPERIT_LLAMA_TEST_MODEL pointing to downloaded GGUF weights"]
    fn nativeLlamaCppGeneratesText() {
        let path = std::env::var("OPERIT_LLAMA_TEST_MODEL").expect("Set OPERIT_LLAMA_TEST_MODEL");
        let response = nativeGenerate("llama-cpp", &path,
            &json!({"messages":[{"role":"user","content":"Say hello in one short sentence."}],"max_tokens":16,"temperature":0.0}),
            &json!({"llamaUseMmap":true,"llamaContextSize":512,"llamaBatchSize":128,"llamaUBatchSize":128})).unwrap();
        println!("Native response: {response:?}");
        assert!(!response.text.trim().is_empty());
        assert!(response.metadata["inputTokens"].as_i64().unwrap() > 0);
        assert!(response.metadata["outputTokens"].as_i64().unwrap() > 0);
        println!("llama.cpp: {}", response.text);
        unloadLocalChat();
    }

    #[cfg(all(feature = "native-llm", not(target_arch = "wasm32")))]
    #[test]
    #[ignore = "requires OPERIT_MNN_TEST_CONFIG pointing to a downloaded MNN model bundle"]
    fn nativeMnnGeneratesText() {
        let path = std::env::var("OPERIT_MNN_TEST_CONFIG").expect("Set OPERIT_MNN_TEST_CONFIG");
        let response = nativeGenerate("mnn", &path,
            &json!({"messages":[{"role":"user","content":"Say hello in one short sentence."}],"max_tokens":16,"temperature":0.0}), &json!({})).unwrap();
        println!("Native response: {response:?}");
        assert!(!response.text.trim().is_empty());
        assert!(response.metadata["inputTokens"].as_i64().unwrap() > 0);
        assert!(response.metadata["outputTokens"].as_i64().unwrap() > 0);
        println!("MNN: {}", response.text);
        unloadLocalChat();
    }

    #[cfg(all(feature = "native-llm", not(target_arch = "wasm32")))]
    #[test]
    fn nativeMnnRejectsIncompleteBundlesBeforeLoading() {
        let directory = std::env::temp_dir().join(format!(
            "operit-mnn-validation-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        for file in ["config.json", "llm_config.json"] {
            std::fs::write(directory.join(file), "{}").unwrap();
        }
        for file in ["llm.mnn", "llm.mnn.weight", "tokenizer.txt"] {
            std::fs::write(directory.join(file), "placeholder").unwrap();
        }
        let request = json!({"messages":[{"role":"user","content":"Hello"}]});
        let path = directory.join("config.json");
        let error =
            nativeGenerate("mnn", &path.to_string_lossy(), &request, &json!({})).unwrap_err();
        assert!(error.contains("embeddings_bf16.bin"), "{error}");
        std::fs::write(&path, r#"{"base_dir":"../"}"#).unwrap();
        let error =
            nativeGenerate("mnn", &path.to_string_lossy(), &request, &json!({})).unwrap_err();
        assert!(error.contains("base_dir"), "{error}");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejectsUnboundedOrInvalidInferenceRequests() {
        assert!(validateChatRequest(&json!({"messages": []}), &json!({})).is_err());
        assert!(validateChatRequest(
            &json!({"messages": [{"role":"user","content":"Hi"}],"max_tokens":-1}),
            &json!({})
        )
        .is_err());
        assert!(validateChatRequest(
            &json!({"messages": [{"role":"user","content":"Hi"}]}),
            &json!({"llamaBatchSize":0})
        )
        .is_err());
        assert!(validateChatRequest(
            &json!({"messages": [{"role":"user","content":"Hi"}]}),
            &json!({})
        )
        .is_ok());
    }
}
