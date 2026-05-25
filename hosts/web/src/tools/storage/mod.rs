use js_sys::{Array, Uint8Array};
use operit_host_api::{
    HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
    RuntimeStorageEntry, RuntimeStorageHost, SqliteRow, SqliteValue,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    bytes_to_js, call_sqlite, call_storage, js_bool, js_i64, js_rows, js_string, js_usize,
    read_bool_property, read_i64_property, read_string_property, sqlite_params_to_js,
};

#[derive(Clone, Debug, Default)]
pub struct WebRuntimeStorageHost;

unsafe impl Send for WebRuntimeStorageHost {}
unsafe impl Sync for WebRuntimeStorageHost {}

impl WebRuntimeStorageHost {
    pub fn new() -> Self {
        Self
    }
}

impl RuntimeStorageHost for WebRuntimeStorageHost {
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        let value = call_storage("readBytes", &[JsValue::from_str(path)])?;
        Ok(Uint8Array::new(&value).to_vec())
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        call_storage("writeBytes", &[JsValue::from_str(path), bytes_to_js(content)])?;
        Ok(())
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        call_storage(
            "delete",
            &[JsValue::from_str(path), JsValue::from_bool(recursive)],
        )?;
        Ok(())
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        js_bool(call_storage("exists", &[JsValue::from_str(path)])?, "runtimeStorage.exists")
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let value = call_storage("list", &[JsValue::from_str(prefix)])?;
        let array = Array::from(&value);
        let mut entries = Vec::new();
        for index in 0..array.length() {
            let entry = array.get(index);
            entries.push(RuntimeStorageEntry {
                path: read_string_property(&entry, "path")?,
                isDirectory: read_bool_property(&entry, "isDirectory")?,
                size: read_i64_property(&entry, "size")?,
            });
        }
        Ok(entries)
    }
}

impl RuntimeSqliteHost for WebRuntimeStorageHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        let id = call_sqlite("open", &[JsValue::from_str(path)])?;
        Ok(Box::new(WebRuntimeSqliteConnection {
            id: js_string(id, "sqlite.open")?,
        }))
    }
}

struct WebRuntimeSqliteConnection {
    id: String,
}

unsafe impl Send for WebRuntimeSqliteConnection {}

impl RuntimeSqliteConnection for WebRuntimeSqliteConnection {
    fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
        call_sqlite(
            "executeBatch",
            &[JsValue::from_str(&self.id), JsValue::from_str(sql)],
        )?;
        Ok(())
    }

    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let value = call_sqlite(
            "execute",
            &[
                JsValue::from_str(&self.id),
                JsValue::from_str(sql),
                sqlite_params_to_js(params),
            ],
        )?;
        js_usize(value, "sqlite.execute")
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        let value = call_sqlite(
            "query",
            &[
                JsValue::from_str(&self.id),
                JsValue::from_str(sql),
                sqlite_params_to_js(params),
            ],
        )?;
        js_rows(value)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        let value = call_sqlite("lastInsertRowId", &[JsValue::from_str(&self.id)])?;
        js_i64(value, "sqlite.lastInsertRowId")
    }

    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
        let id = call_sqlite("beginTransaction", &[JsValue::from_str(&self.id)])?;
        Ok(Box::new(WebRuntimeSqliteTransaction {
            id: js_string(id, "sqlite.beginTransaction")?,
        }))
    }
}

struct WebRuntimeSqliteTransaction {
    id: String,
}

unsafe impl Send for WebRuntimeSqliteTransaction {}

impl RuntimeSqliteTransaction for WebRuntimeSqliteTransaction {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let value = call_sqlite(
            "transactionExecute",
            &[
                JsValue::from_str(&self.id),
                JsValue::from_str(sql),
                sqlite_params_to_js(params),
            ],
        )?;
        js_usize(value, "sqlite.transactionExecute")
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        let value = call_sqlite(
            "transactionQuery",
            &[
                JsValue::from_str(&self.id),
                JsValue::from_str(sql),
                sqlite_params_to_js(params),
            ],
        )?;
        js_rows(value)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        let value = call_sqlite("transactionLastInsertRowId", &[JsValue::from_str(&self.id)])?;
        js_i64(value, "sqlite.transactionLastInsertRowId")
    }

    fn commit(self: Box<Self>) -> HostResult<()> {
        call_sqlite("commitTransaction", &[JsValue::from_str(&self.id)])?;
        Ok(())
    }
}
