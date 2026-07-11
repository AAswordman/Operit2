use operit_host_api::{
    HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    call_managed_runtime, call_managed_runtime_process, js_bool, js_optional_string, js_string,
    optional_str_to_js, program_to_js, read_optional_i32_property, read_string_property,
    runtime_process_request_to_js,
};

#[derive(Clone, Debug, Default)]
pub struct WebManagedRuntimeHost;

unsafe impl Send for WebManagedRuntimeHost {}
unsafe impl Sync for WebManagedRuntimeHost {}

impl WebManagedRuntimeHost {
    pub fn new() -> Self {
        Self
    }
}

impl ManagedRuntimeHost for WebManagedRuntimeHost {
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        js_string(
            call_managed_runtime("runtimeWorkspaceDir", &[])?,
            "runtimeWorkspaceDir",
        )
    }

    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        js_string(
            call_managed_runtime(
                "resolveRuntimeExecutable",
                &[program_to_js(program), optional_str_to_js(executablePath)],
            )?,
            "resolveRuntimeExecutable",
        )
    }

    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let id = js_string(
            call_managed_runtime(
                "startRuntimeProcess",
                &[runtime_process_request_to_js(&request)],
            )?,
            "startRuntimeProcess",
        )?;
        Ok(Box::new(WebManagedRuntimeProcess { id }))
    }

    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let value = call_managed_runtime(
            "runRuntimeCommand",
            &[runtime_process_request_to_js(&request)],
        )?;
        Ok(RuntimeCommandOutput {
            exitCode: read_optional_i32_property(&value, "exitCode")?,
            stdout: read_string_property(&value, "stdout")?,
            stderr: read_string_property(&value, "stderr")?,
        })
    }
}

struct WebManagedRuntimeProcess {
    id: String,
}

unsafe impl Send for WebManagedRuntimeProcess {}

impl ManagedRuntimeProcess for WebManagedRuntimeProcess {
    fn writeLine(&self, line: &str) -> HostResult<()> {
        call_managed_runtime_process(
            "writeLine",
            &[JsValue::from_str(&self.id), JsValue::from_str(line)],
        )?;
        Ok(())
    }

    /// Writes multiple protocol lines to the managed runtime process.
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        let value = js_sys::Array::new();
        for line in lines {
            value.push(&JsValue::from_str(line));
        }
        call_managed_runtime_process("writeLines", &[JsValue::from_str(&self.id), value.into()])?;
        Ok(())
    }

    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let value = call_managed_runtime_process(
            "readStdoutLine",
            &[
                JsValue::from_str(&self.id),
                JsValue::from_f64(timeoutMs as f64),
            ],
        )?;
        js_optional_string(value, "readStdoutLine")
    }

    fn drainStderr(&self) -> HostResult<String> {
        js_string(
            call_managed_runtime_process("drainStderr", &[JsValue::from_str(&self.id)])?,
            "drainStderr",
        )
    }

    fn isRunning(&self) -> HostResult<bool> {
        js_bool(
            call_managed_runtime_process("isRunning", &[JsValue::from_str(&self.id)])?,
            "isRunning",
        )
    }

    fn kill(&self) -> HostResult<()> {
        call_managed_runtime_process("kill", &[JsValue::from_str(&self.id)])?;
        Ok(())
    }
}
