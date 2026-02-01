use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use objc2::{class, msg_send};
use objc2::runtime::AnyObject;

pub fn nsstring_from_str(value: &str) -> Result<*mut AnyObject, String> {
    let cstring = CString::new(value)
        .map_err(|error| format!("invalid string for NSString: {error}"))?;
    let nsstring: *mut AnyObject = unsafe {
        msg_send![class!(NSString), stringWithUTF8String: cstring.as_ptr()]
    };
    if nsstring.is_null() {
        return Err("failed to create NSString".to_string());
    }
    Ok(nsstring)
}

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
