use std::ffi::c_void;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::{CGPoint, CGSize};

use super::Rect;

pub type AXUIElementRef = *const c_void;

type AXError = i32;
const AX_ERROR_SUCCESS: AXError = 0;

#[allow(non_camel_case_types)]
type AXValueType = i32;
const K_AX_VALUE_CGPOINT_TYPE: AXValueType = 1;
const K_AX_VALUE_CGSIZE_TYPE: AXValueType = 2;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXValueGetValue(value: CFTypeRef, value_type: AXValueType, output: *mut c_void) -> bool;
}

fn ax_windows_attribute() -> CFString {
    CFString::from_static_string("AXWindows")
}

fn ax_title_attribute() -> CFString {
    CFString::from_static_string("AXTitle")
}

fn ax_minimized_attribute() -> CFString {
    CFString::from_static_string("AXMinimized")
}

fn ax_position_attribute() -> CFString {
    CFString::from_static_string("AXPosition")
}

fn ax_size_attribute() -> CFString {
    CFString::from_static_string("AXSize")
}

fn ax_close_button_attribute() -> CFString {
    CFString::from_static_string("AXCloseButton")
}

fn ax_raise_action() -> CFString {
    CFString::from_static_string("AXRaise")
}

fn ax_press_action() -> CFString {
    CFString::from_static_string("AXPress")
}

fn ax_frontmost_attribute() -> CFString {
    CFString::from_static_string("AXFrontmost")
}

#[derive(Debug, Clone)]
pub struct AxElement {
    inner: CFType,
}

impl AxElement {
    fn new(inner: CFType) -> Self {
        Self { inner }
    }

    pub fn as_ref(&self) -> AXUIElementRef {
        self.inner.as_CFTypeRef() as AXUIElementRef
    }
}

pub fn create_application(pid: i32) -> Result<AxElement, String> {
    let app_ref = unsafe { AXUIElementCreateApplication(pid) };
    if app_ref.is_null() {
        return Err("failed to create AX application element".to_string());
    }
    let app = unsafe { CFType::wrap_under_create_rule(app_ref as CFTypeRef) };
    Ok(AxElement::new(app))
}

pub fn copy_windows(app: &AxElement) -> Result<Vec<AxElement>, String> {
    let mut value: CFTypeRef = std::ptr::null();
    let attribute = ax_windows_attribute();
    let error = unsafe {
        AXUIElementCopyAttributeValue(
            app.as_ref(),
            attribute.as_CFTypeRef() as CFStringRef,
            &mut value,
        )
    };
    if error != AX_ERROR_SUCCESS || value.is_null() {
        return Ok(Vec::new());
    }
    let array = unsafe { CFArray::<CFType>::wrap_under_create_rule(value as CFArrayRef) };
    Ok(array
        .iter()
        .map(|item| AxElement::new(item.clone()))
        .collect())
}

pub fn copy_title(element: &AxElement) -> Option<String> {
    copy_attribute_value_optional(element, ax_title_attribute().as_CFTypeRef() as CFStringRef)
        .and_then(|value| value.downcast::<CFString>().map(|value| value.to_string()))
}

pub fn copy_minimized(element: &AxElement) -> Option<bool> {
    copy_attribute_value_optional(
        element,
        ax_minimized_attribute().as_CFTypeRef() as CFStringRef,
    )
    .and_then(|value| value.downcast::<CFBoolean>().map(bool::from))
}

pub fn copy_bounds(element: &AxElement) -> Option<Rect> {
    let position = copy_attribute_value_optional(
        element,
        ax_position_attribute().as_CFTypeRef() as CFStringRef,
    )
    .and_then(ax_value_to_point);
    let size =
        copy_attribute_value_optional(element, ax_size_attribute().as_CFTypeRef() as CFStringRef)
            .and_then(ax_value_to_size);
    match (position, size) {
        (Some(position), Some(size)) => Some(Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }),
        _ => None,
    }
}

pub fn perform_raise(element: &AxElement) -> Result<(), String> {
    perform_action(element, ax_raise_action().as_CFTypeRef() as CFStringRef)
}

pub fn perform_press(element: &AxElement) -> Result<(), String> {
    perform_action(element, ax_press_action().as_CFTypeRef() as CFStringRef)
}

pub fn set_minimized(element: &AxElement, minimized: bool) -> Result<(), String> {
    let value = if minimized {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    };
    set_attribute_value(
        element,
        ax_minimized_attribute().as_CFTypeRef() as CFStringRef,
        value.as_CFTypeRef(),
    )
}

pub fn set_frontmost(app: &AxElement, frontmost: bool) -> Result<(), String> {
    let value = if frontmost {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    };
    set_attribute_value(
        app,
        ax_frontmost_attribute().as_CFTypeRef() as CFStringRef,
        value.as_CFTypeRef(),
    )
}

pub fn copy_close_button(element: &AxElement) -> Option<AxElement> {
    copy_attribute_value_optional(
        element,
        ax_close_button_attribute().as_CFTypeRef() as CFStringRef,
    )
    .map(|value| AxElement::new(value))
}

fn perform_action(element: &AxElement, action: CFStringRef) -> Result<(), String> {
    let error = unsafe { AXUIElementPerformAction(element.as_ref(), action) };
    if error != AX_ERROR_SUCCESS {
        return Err(format!("failed to perform AX action (error {error})"));
    }
    Ok(())
}

fn set_attribute_value(
    element: &AxElement,
    attribute: CFStringRef,
    value: CFTypeRef,
) -> Result<(), String> {
    let error = unsafe { AXUIElementSetAttributeValue(element.as_ref(), attribute, value) };
    if error != AX_ERROR_SUCCESS {
        return Err(format!("failed to set AX attribute (error {error})"));
    }
    Ok(())
}

fn copy_attribute_value_optional(element: &AxElement, attribute: CFStringRef) -> Option<CFType> {
    let mut value: CFTypeRef = std::ptr::null();
    let error = unsafe { AXUIElementCopyAttributeValue(element.as_ref(), attribute, &mut value) };
    if error != AX_ERROR_SUCCESS || value.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(value) })
}

fn ax_value_to_point(value: CFType) -> Option<CGPoint> {
    let mut point = CGPoint { x: 0.0, y: 0.0 };
    let success = unsafe {
        AXValueGetValue(
            value.as_CFTypeRef(),
            K_AX_VALUE_CGPOINT_TYPE,
            &mut point as *mut _ as *mut c_void,
        )
    };
    if success {
        Some(point)
    } else {
        None
    }
}

fn ax_value_to_size(value: CFType) -> Option<CGSize> {
    let mut size = CGSize {
        width: 0.0,
        height: 0.0,
    };
    let success = unsafe {
        AXValueGetValue(
            value.as_CFTypeRef(),
            K_AX_VALUE_CGSIZE_TYPE,
            &mut size as *mut _ as *mut c_void,
        )
    };
    if success {
        Some(size)
    } else {
        None
    }
}
