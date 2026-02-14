use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::CGRect;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowBounds, kCGWindowIsOnscreen, kCGWindowLayer, kCGWindowListOptionAll,
    kCGWindowListOptionOnScreenOnly, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID, CGWindowListCopyWindowInfo,
};

use super::{classify_window_kind, Rect, WindowInfo, WindowState};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> bool;
}

pub fn list_windows_cg(visible_only: bool) -> Result<Vec<WindowInfo>, String> {
    let options = if visible_only {
        kCGWindowListOptionOnScreenOnly
    } else {
        kCGWindowListOptionAll
    };
    let array_ref = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };
    if array_ref.is_null() {
        return Ok(Vec::new());
    }
    let array: CFArray<CFDictionary<CFType, CFType>> =
        unsafe { TCFType::wrap_under_create_rule(array_ref) };

    let mut windows = Vec::new();
    for dict in array.iter() {
        let window_id = unsafe { dict_get_i64(&dict, kCGWindowNumber) }.unwrap_or(0) as u32;
        let owner_pid = unsafe { dict_get_i64(&dict, kCGWindowOwnerPID) }.unwrap_or(0) as i32;
        let owner_name = unsafe { dict_get_string(&dict, kCGWindowOwnerName) };
        let title = unsafe { dict_get_string(&dict, kCGWindowName) };
        let is_onscreen = unsafe { dict_get_bool(&dict, kCGWindowIsOnscreen) }.unwrap_or(false);
        let layer = unsafe { dict_get_i64(&dict, kCGWindowLayer) }.unwrap_or(0) as i32;
        let bounds = unsafe { dict_get_rect(&dict, kCGWindowBounds) }.unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });

        if window_id == 0 || owner_pid == 0 {
            continue;
        }

        let mut window = WindowInfo {
            window_id,
            title,
            bounds,
            layer,
            is_onscreen,
            minimized: None,
            state: if is_onscreen {
                WindowState::Onscreen
            } else {
                WindowState::Unknown
            },
            kind: classify_window_kind(layer),
            owner_pid,
            owner_name,
        };
        window.update_state();
        windows.push(window);
    }

    Ok(windows)
}

unsafe fn dict_get_string(dict: &CFDictionary<CFType, CFType>, key: CFStringRef) -> Option<String> {
    let key = CFString::wrap_under_get_rule(key);
    let value = dict.find(key.as_CFType())?;
    value.downcast::<CFString>().map(|value| value.to_string())
}

unsafe fn dict_get_i64(dict: &CFDictionary<CFType, CFType>, key: CFStringRef) -> Option<i64> {
    let key = CFString::wrap_under_get_rule(key);
    let value = dict.find(key.as_CFType())?;
    value
        .downcast::<CFNumber>()
        .and_then(|value| value.to_i64())
}

unsafe fn dict_get_bool(dict: &CFDictionary<CFType, CFType>, key: CFStringRef) -> Option<bool> {
    let key = CFString::wrap_under_get_rule(key);
    let value = dict.find(key.as_CFType())?;
    value.downcast::<CFBoolean>().map(bool::from)
}

unsafe fn dict_get_rect(dict: &CFDictionary<CFType, CFType>, key: CFStringRef) -> Option<Rect> {
    let key = CFString::wrap_under_get_rule(key);
    let value = dict.find(key.as_CFType())?;
    let rect_dict = value.as_CFTypeRef() as CFDictionaryRef;
    let mut rect = CGRect::new(
        &core_graphics::geometry::CGPoint::new(0.0, 0.0),
        &core_graphics::geometry::CGSize::new(0.0, 0.0),
    );
    let success = unsafe { CGRectMakeWithDictionaryRepresentation(rect_dict, &mut rect) };
    if !success {
        return None;
    }
    Some(Rect {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    })
}
