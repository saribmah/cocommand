use std::ffi::CStr;
use std::os::raw::c_char;

use objc2::{msg_send};
use objc2::runtime::AnyObject;

pub fn nsstring_to_string(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let cstr: *const c_char = unsafe { msg_send![value, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(cstr) };
    Some(value.to_string_lossy().to_string())
}
