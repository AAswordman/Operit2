use base64::Engine;
use operit_host_api::FileSystemResource::FileSystemResource;
use serde_json::{json, Value};
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
};

#[derive(Clone, Debug, Default)]
pub struct AndroidFileSystemHost {
    inner: operit_host_native_common::PosixFileSystemHost,
}

impl AndroidFileSystemHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_native_common::PosixFileSystemHost::new(),
        }
    }
}

impl FileSystemHost for AndroidFileSystemHost {
    fn envLabel(&self) -> &str {
        "android"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::android()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        if let Some(resource) = FileSystemResource::parse(path)? {
            if resource.backend != "android_documents" { return Err(HostError::new("Unsupported Android filesystem backend")); }
            return Ok(());
        }
        if !std::path::Path::new(path).is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Android path."
            )));
        }
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "listFiles", "path": path}))?;
            return serde_json::from_value(value).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.listFiles(path)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "readFileBytes", "path": path}))?;
            return String::from_utf8(decodeBytes(value)?).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.readFile(path)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "readFileWithLimit", "path": path, "maxBytes": maxBytes}))?;
            return Ok(String::from_utf8_lossy(&decodeBytes(value)?).into_owned());
        }
        self.inner.readFileWithLimit(path, maxBytes)
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "readFileBytes", "path": path}))?;
            return decodeBytes(value);
        }
        self.inner.readFileBytes(path)
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "writeFile", "path": path, "content": base64::engine::general_purpose::STANDARD.encode(content.as_bytes()), "append": append}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.writeFile(path, content, append)
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "writeFileBytes", "path": path, "content": base64::engine::general_purpose::STANDARD.encode(content)}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.writeFileBytes(path, content)
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "deleteFile", "path": path, "recursive": recursive}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.deleteFile(path, recursive)
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "fileExists", "path": path}))?;
            return serde_json::from_value(value).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.fileExists(path)
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        if isResource(source)? || isResource(destination)? {
            let value = crate::document_filesystem::call(json!({"operation": "moveFile", "source": source, "destination": destination, "recursive": true}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.moveFile(source, destination)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        if isResource(source)? || isResource(destination)? {
            let value = crate::document_filesystem::call(json!({"operation": "copyFile", "source": source, "destination": destination, "recursive": recursive}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.copyFile(source, destination, recursive)
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "makeDirectory", "path": path, "createParents": createParents}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.makeDirectory(path, createParents)
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        if isResource(&request.path)? {
            let mut payload = serde_json::to_value(&request).map_err(|e| HostError::new(e.to_string()))?;
            payload["operation"] = json!("findFiles");
            let value = crate::document_filesystem::call(payload)?;
            return serde_json::from_value(value).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.findFiles(request)
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        if isResource(path)? {
            let value = crate::document_filesystem::call(json!({"operation": "fileInfo", "path": path}))?;
            return serde_json::from_value(value).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.fileInfo(path)
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        if isResource(&request.path)? {
            let mut payload = serde_json::to_value(&request).map_err(|e| HostError::new(e.to_string()))?;
            payload["operation"] = json!("grepCode");
            let value = crate::document_filesystem::call(payload)?;
            return serde_json::from_value(value).map_err(|e| HostError::new(e.to_string()));
        }
        self.inner.grepCode(request)
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        if isResource(source)? || isResource(destination)? {
            let value = crate::document_filesystem::call(json!({"operation": "zipFiles", "source": source, "destination": destination}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.zipFiles(source, destination)
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        if isResource(source)? || isResource(destination)? {
            let value = crate::document_filesystem::call(json!({"operation": "unzipFiles", "source": source, "destination": destination}))?;
            return if value.is_null() { Ok(()) } else { Err(HostError::new("Invalid document operation response")) };
        }
        self.inner.unzipFiles(source, destination)
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        self.validatePath(path, "path")?;
        let value = crate::document_filesystem::call(json!({
            "operation": "openFile", "path": path
        }))?;
        if value.is_null() {
            Ok(())
        } else {
            Err(HostError::new("Invalid Android open-file response"))
        }
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android share_file requires the Flutter Android host bridge: {path} ({title})"
        )))
    }
}

fn isResource(path: &str) -> HostResult<bool> {
    Ok(FileSystemResource::parse(path)?.is_some())
}
fn decodeBytes(value: Value) -> HostResult<Vec<u8>> {
    let text = value.as_str().ok_or_else(|| HostError::new("Invalid document byte response"))?;
    base64::engine::general_purpose::STANDARD.decode(text).map_err(|e| HostError::new(e.to_string()))
}
