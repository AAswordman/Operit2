use std::ffi::CString;
use std::sync::Arc;

use operit_host_api::setHostConsoleLogSink;

#[link(name = "log")]
extern "C" {
    fn __android_log_write(
        priority: libc::c_int,
        tag: *const libc::c_char,
        text: *const libc::c_char,
    ) -> libc::c_int;
}

/// Installs the Android Logcat sink for all runtime log records.
pub fn installAndroidLogSink() {
    setHostConsoleLogSink(Arc::new(|priority, _tag, message| {
        let Ok(tag) = CString::new("OperitRust") else {
            return;
        };
        let Ok(text) = CString::new(message) else {
            return;
        };
        unsafe {
            let _ = __android_log_write(priority, tag.as_ptr(), text.as_ptr());
        }
    }));
}
