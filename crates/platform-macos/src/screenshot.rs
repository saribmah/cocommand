use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum ScreenshotMode {
    Interactive,
    Screen,
    Window,
    Rect,
}

#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    pub mode: ScreenshotMode,
    pub display: Option<u32>,
    pub window_id: Option<u32>,
    pub rect: Option<String>,
    pub format: Option<String>,
    pub delay_seconds: Option<u64>,
    pub to_clipboard: bool,
    pub include_cursor: bool,
}

#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub path: Option<String>,
    pub filename: Option<String>,
    pub format: String,
    pub clipboard: bool,
}

pub fn capture_screenshot(
    options: ScreenshotOptions,
    output_path: Option<&Path>,
) -> Result<ScreenshotResult, String> {
    let mut command = Command::new("/usr/sbin/screencapture");

    match options.mode {
        ScreenshotMode::Interactive => {
            command.arg("-i");
        }
        ScreenshotMode::Screen => {}
        ScreenshotMode::Window => {
            if let Some(window_id) = options.window_id {
                command.arg(format!("-l{}", window_id));
            } else {
                return Err("windowId is required for window capture".to_string());
            }
        }
        ScreenshotMode::Rect => {
            if let Some(rect) = &options.rect {
                command.arg(format!("-R{}", rect));
            } else {
                return Err("rect is required for rect capture".to_string());
            }
        }
    }

    if let Some(display) = options.display {
        command.arg(format!("-D{}", display));
    }

    if let Some(format) = &options.format {
        command.arg(format!("-t{}", format));
    }

    if let Some(delay) = options.delay_seconds {
        command.arg(format!("-T{}", delay));
    }

    if options.to_clipboard {
        command.arg("-c");
    }

    if options.include_cursor {
        command.arg("-C");
    }

    let mut output_path_value = None;
    let mut filename = None;
    if let Some(path) = output_path {
        command.arg(path);
        output_path_value = Some(path.to_string_lossy().to_string());
        filename = path.file_name().map(|name| name.to_string_lossy().to_string());
    }

    let status = command.status().map_err(|error| {
        format!("failed to run screencapture: {error}")
    })?;

    if !status.success() {
        return Err(format!("screencapture failed with status {status}"));
    }

    if let Some(path) = output_path {
        if !path.exists() {
            output_path_value = None;
            filename = None;
        }
    }

    Ok(ScreenshotResult {
        path: output_path_value,
        filename,
        format: options.format.unwrap_or_else(|| "png".to_string()),
        clipboard: options.to_clipboard,
    })
}
