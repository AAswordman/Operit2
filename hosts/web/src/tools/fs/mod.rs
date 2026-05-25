use js_sys::{Array, Reflect, Uint8Array};
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, GrepLineMatch, HostEnvironmentDescriptor, HostResult,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    bytes_to_js, call_file_system, find_files_request_to_js, grep_code_request_to_js, js_error,
    js_string, js_string_array, read_bool_property, read_i64_property,
    read_optional_string_property, read_string_property, read_usize_property,
};

#[derive(Clone, Debug, Default)]
pub struct WebFileSystemHost;

unsafe impl Send for WebFileSystemHost {}
unsafe impl Sync for WebFileSystemHost {}

impl WebFileSystemHost {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemHost for WebFileSystemHost {
    fn envLabel(&self) -> &str {
        "web"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::web()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        call_file_system(
            "validatePath",
            &[JsValue::from_str(path), JsValue::from_str(paramName)],
        )?;
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        let value = call_file_system("listFiles", &[JsValue::from_str(path)])?;
        let array = Array::from(&value);
        let mut entries = Vec::new();
        for index in 0..array.length() {
            let entry = array.get(index);
            entries.push(FileEntry {
                name: read_string_property(&entry, "name")?,
                isDirectory: read_bool_property(&entry, "isDirectory")?,
                size: read_i64_property(&entry, "size")?,
                permissions: read_string_property(&entry, "permissions")?,
                lastModified: read_string_property(&entry, "lastModified")?,
            });
        }
        Ok(entries)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        js_string(call_file_system("readFile", &[JsValue::from_str(path)])?, "readFile")
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        js_string(
            call_file_system(
                "readFileWithLimit",
                &[JsValue::from_str(path), JsValue::from_f64(maxBytes as f64)],
            )?,
            "readFileWithLimit",
        )
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        let value = call_file_system("readFileBytes", &[JsValue::from_str(path)])?;
        Ok(Uint8Array::new(&value).to_vec())
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        call_file_system(
            "writeFile",
            &[
                JsValue::from_str(path),
                JsValue::from_str(content),
                JsValue::from_bool(append),
            ],
        )?;
        Ok(())
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        call_file_system("writeFileBytes", &[JsValue::from_str(path), bytes_to_js(content)])?;
        Ok(())
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        call_file_system(
            "deleteFile",
            &[JsValue::from_str(path), JsValue::from_bool(recursive)],
        )?;
        Ok(())
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        let value = call_file_system("fileExists", &[JsValue::from_str(path)])?;
        Ok(FileExistence {
            exists: read_bool_property(&value, "exists")?,
            isDirectory: read_bool_property(&value, "isDirectory")?,
            size: read_i64_property(&value, "size")?,
        })
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        call_file_system(
            "moveFile",
            &[JsValue::from_str(source), JsValue::from_str(destination)],
        )?;
        Ok(())
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        call_file_system(
            "copyFile",
            &[
                JsValue::from_str(source),
                JsValue::from_str(destination),
                JsValue::from_bool(recursive),
            ],
        )?;
        Ok(())
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        call_file_system(
            "makeDirectory",
            &[JsValue::from_str(path), JsValue::from_bool(createParents)],
        )?;
        Ok(())
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        let value = call_file_system("findFiles", &[find_files_request_to_js(&request)])?;
        js_string_array(value, "findFiles")
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        let value = call_file_system("fileInfo", &[JsValue::from_str(path)])?;
        Ok(FileInfo {
            path: read_string_property(&value, "path")?,
            exists: read_bool_property(&value, "exists")?,
            fileType: read_string_property(&value, "fileType")?,
            size: read_i64_property(&value, "size")?,
            permissions: read_string_property(&value, "permissions")?,
            owner: read_string_property(&value, "owner")?,
            group: read_string_property(&value, "group")?,
            lastModified: read_string_property(&value, "lastModified")?,
            rawStatOutput: read_string_property(&value, "rawStatOutput")?,
        })
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        let value = call_file_system("grepCode", &[grep_code_request_to_js(&request)])?;
        let matches = Array::from(&Reflect::get(&value, &JsValue::from_str("matches")).map_err(js_error)?);
        let mut grep_matches = Vec::new();
        for index in 0..matches.length() {
            let file_match = matches.get(index);
            let line_matches = Array::from(
                &Reflect::get(&file_match, &JsValue::from_str("lineMatches")).map_err(js_error)?,
            );
            let mut parsed_line_matches = Vec::new();
            for line_index in 0..line_matches.length() {
                let line_match = line_matches.get(line_index);
                parsed_line_matches.push(GrepLineMatch {
                    lineNumber: read_usize_property(&line_match, "lineNumber")?,
                    lineContent: read_string_property(&line_match, "lineContent")?,
                    matchContext: read_optional_string_property(&line_match, "matchContext")?,
                });
            }
            grep_matches.push(GrepFileMatch {
                filePath: read_string_property(&file_match, "filePath")?,
                lineMatches: parsed_line_matches,
            });
        }
        Ok(GrepCodeResult {
            matches: grep_matches,
            totalMatches: read_usize_property(&value, "totalMatches")?,
            filesSearched: read_usize_property(&value, "filesSearched")?,
        })
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        call_file_system(
            "zipFiles",
            &[JsValue::from_str(source), JsValue::from_str(destination)],
        )?;
        Ok(())
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        call_file_system(
            "unzipFiles",
            &[JsValue::from_str(source), JsValue::from_str(destination)],
        )?;
        Ok(())
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        call_file_system("openFile", &[JsValue::from_str(path)])?;
        Ok(())
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        call_file_system("shareFile", &[JsValue::from_str(path), JsValue::from_str(title)])?;
        Ok(())
    }
}
