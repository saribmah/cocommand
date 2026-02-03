use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::{CGPoint, CGRect};

#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn point_in_rect(point: CGPoint, rect: CGRect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.height
}

pub fn active_screen_visible_frame() -> Option<ScreenFrame> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let mouse = CGEvent::new(source).ok()?.location();

    let displays = CGDisplay::active_displays().ok()?;
    for display_id in displays {
        let display = CGDisplay::new(display_id);
        let bounds = display.bounds();
        if point_in_rect(mouse, bounds) {
            return Some(ScreenFrame {
                x: bounds.origin.x as f64,
                y: bounds.origin.y as f64,
                width: bounds.size.width as f64,
                height: bounds.size.height as f64,
            });
        }
    }
    None
}
