use objc2::rc::autoreleasepool;
use objc2::{class, msg_send};
use objc2::runtime::{AnyObject, Bool};

use crate::util::{nsdata_to_vec, nsstring_to_string, string_to_nsstring, vec_to_nsdata};

#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Text(String),
    Image(Vec<u8>),
    Files(Vec<String>),
}

pub fn clipboard_change_count() -> Result<i64, String> {
    autoreleasepool(|_| {
        let pasteboard: *mut AnyObject =
            unsafe { msg_send![class!(NSPasteboard), generalPasteboard] };
        if pasteboard.is_null() {
            return Err("failed to access NSPasteboard".to_string());
        }
        let count: i64 = unsafe { msg_send![pasteboard, changeCount] };
        Ok(count)
    })
}

pub fn read_clipboard() -> Result<Option<ClipboardItem>, String> {
    autoreleasepool(|_| {
        let pasteboard: *mut AnyObject =
            unsafe { msg_send![class!(NSPasteboard), generalPasteboard] };
        if pasteboard.is_null() {
            return Err("failed to access NSPasteboard".to_string());
        }

        let filenames_type = string_to_nsstring("NSFilenamesPboardType");
        let filenames: *mut AnyObject =
            unsafe { msg_send![pasteboard, propertyListForType: filenames_type] };
        if !filenames.is_null() {
            let count: usize = unsafe { msg_send![filenames, count] };
            if count > 0 {
                let mut files = Vec::with_capacity(count);
                for index in 0..count {
                    let item: *mut AnyObject = unsafe { msg_send![filenames, objectAtIndex: index] };
                    if let Some(value) = nsstring_to_string(item) {
                        files.push(value);
                    }
                }
                if !files.is_empty() {
                    return Ok(Some(ClipboardItem::Files(files)));
                }
            }
        }

        let tiff_type = string_to_nsstring("public.tiff");
        let image_data: *mut AnyObject = unsafe { msg_send![pasteboard, dataForType: tiff_type] };
        if let Some(bytes) = nsdata_to_vec(image_data) {
            if !bytes.is_empty() {
                return Ok(Some(ClipboardItem::Image(bytes)));
            }
        }

        let utf8_type = string_to_nsstring("public.utf8-plain-text");
        let text: *mut AnyObject = unsafe { msg_send![pasteboard, stringForType: utf8_type] };
        if let Some(value) = nsstring_to_string(text) {
            return Ok(Some(ClipboardItem::Text(value)));
        }

        let legacy_type = string_to_nsstring("NSStringPboardType");
        let text: *mut AnyObject = unsafe { msg_send![pasteboard, stringForType: legacy_type] };
        if let Some(value) = nsstring_to_string(text) {
            return Ok(Some(ClipboardItem::Text(value)));
        }

        Ok(None)
    })
}

pub fn write_clipboard(item: ClipboardItem) -> Result<(), String> {
    autoreleasepool(|_| {
        let pasteboard: *mut AnyObject =
            unsafe { msg_send![class!(NSPasteboard), generalPasteboard] };
        if pasteboard.is_null() {
            return Err("failed to access NSPasteboard".to_string());
        }
        let _: i32 = unsafe { msg_send![pasteboard, clearContents] };

        match item {
            ClipboardItem::Text(text) => {
                let text = string_to_nsstring(&text);
                let text_type = string_to_nsstring("public.utf8-plain-text");
                let ok: Bool = unsafe { msg_send![pasteboard, setString: text forType: text_type] };
                if bool::from(ok) {
                    Ok(())
                } else {
                    Err("failed to set text clipboard".to_string())
                }
            }
            ClipboardItem::Image(bytes) => {
                let data = vec_to_nsdata(&bytes);
                let tiff_type = string_to_nsstring("public.tiff");
                let ok: Bool = unsafe { msg_send![pasteboard, setData: data forType: tiff_type] };
                if bool::from(ok) {
                    Ok(())
                } else {
                    Err("failed to set image clipboard".to_string())
                }
            }
            ClipboardItem::Files(files) => {
                let array: *mut AnyObject = unsafe { msg_send![class!(NSMutableArray), array] };
                if array.is_null() {
                    return Err("failed to create NSArray".to_string());
                }
                for file in files {
                    let path = string_to_nsstring(&file);
                    let url: *mut AnyObject =
                        unsafe { msg_send![class!(NSURL), fileURLWithPath: path] };
                    if !url.is_null() {
                        let _: () = unsafe { msg_send![array, addObject: url] };
                    }
                }
                let ok: Bool = unsafe { msg_send![pasteboard, writeObjects: array] };
                if bool::from(ok) {
                    Ok(())
                } else {
                    Err("failed to set file clipboard".to_string())
                }
            }
        }
    })
}
