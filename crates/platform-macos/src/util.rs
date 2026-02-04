use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use objc2::class;
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

pub fn string_to_nsstring(value: &str) -> *mut AnyObject {
    let cstring = CString::new(value).unwrap_or_default();
    unsafe { msg_send![class!(NSString), stringWithUTF8String: cstring.as_ptr()] }
}

pub fn nsdata_to_vec(data: *mut AnyObject) -> Option<Vec<u8>> {
    if data.is_null() {
        return None;
    }
    let bytes: *const u8 = unsafe { msg_send![data, bytes] };
    let length: usize = unsafe { msg_send![data, length] };
    if bytes.is_null() {
        return Some(Vec::new());
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, length) };
    Some(slice.to_vec())
}

pub fn vec_to_nsdata(bytes: &[u8]) -> *mut AnyObject {
    unsafe { msg_send![class!(NSData), dataWithBytes: bytes.as_ptr() length: bytes.len()] }
}
