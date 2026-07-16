//! FFI برای فراخوانی از C/Java/Kotlin/Python (ctypes).
//! خروجی به صورت JSON است تا در زبان میزبان پارس شود.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::{run_scan, ScanConfig};

/// اسکن را با مسیر فایل ورودی اجرا می‌کند و نتیجه را به صورت JSON برمی‌گرداند.
///
/// # Safety
/// - `path` باید اشاره‌گر معتبر نول‌ترمینیتد UTF-8 باشد، یا null.
/// - برگشت: اشاره‌گر به رشتهٔ JSON؛ caller باید با `scanner_free_string` آزاد کند.
#[no_mangle]
pub unsafe extern "C" fn scanner_run_from_file(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let path_str = match unsafe { CStr::from_ptr(path).to_str() } {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    let config = ScanConfig {
        input_file: PathBuf::from(&path_str),
        ..ScanConfig::default()
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = rt.block_on(run_scan(config));

    match result {
        Ok(output) => match serde_json::to_string(&output) {
            Ok(json) => match CString::new(json) {
                Ok(c) => c.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// آزاد کردن رشته‌ای که توسط `scanner_run_from_file` برگردانده شده.
///
/// # Safety
/// - `s` باید از `scanner_run_from_file` آمده باشد یا null.
#[no_mangle]
pub unsafe extern "C" fn scanner_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
