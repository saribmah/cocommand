//! Notes add tool.
//!
//! This module provides the tool for adding new notes with automatic categorization.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{categorize_content, escape_applescript_string, run_applescript_with_message};

/// Tool ID for the add tool.
pub const TOOL_ID: &str = "notes_add";

/// Tool for adding a new note.
pub struct NotesAdd;

impl Tool for NotesAdd {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Add Note"
    }

    fn description(&self) -> &str {
        "Add a new note to Apple Notes. The note will be automatically categorized based on its content. Use 'note: <content>' format or provide content directly. Category is auto-assigned but can be overridden."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The content of the note. Can start with 'note:' prefix which will be stripped."
                },
                "title": {
                    "type": "string",
                    "description": "Optional: Custom title for the note. If not provided, uses first line or auto-generates from content."
                },
                "category": {
                    "type": "string",
                    "description": "Optional: Override the auto-detected category. Categories: Work, Personal, Shopping, Ideas, Learning, Health, Finance, General"
                },
                "folder": {
                    "type": "string",
                    "description": "Optional: Name of the Notes folder to add to. Uses default folder if not specified."
                }
            },
            "required": ["content"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let content = inputs
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if content.is_empty() {
            return ToolResult::error("Note content is required");
        }

        // Strip "note:" prefix if present (case insensitive)
        let clean_content = if content.to_lowercase().starts_with("note:") {
            content[5..].trim()
        } else {
            content.trim()
        };

        if clean_content.is_empty() {
            return ToolResult::error("Note content is required (after removing 'note:' prefix)");
        }

        // Auto-categorize or use provided category
        let category = inputs
            .get("category")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .unwrap_or_else(|| categorize_content(clean_content));

        // Generate title from first line or provided title
        let title = inputs
            .get("title")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .unwrap_or_else(|| generate_title(clean_content, &category));

        let folder = inputs
            .get("folder")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let escaped_title = escape_applescript_string(&title);
        let escaped_content = escape_applescript_string(clean_content);

        // Format the body with category tag
        let body_with_category = format!("[{}] {}", category, escaped_content);

        let script = build_add_script(&escaped_title, &body_with_category, folder);

        let success_msg = format!(
            "Note '{}' added to {} category.",
            title, category
        );

        run_applescript_with_message(&script, &success_msg)
    }
}

fn generate_title(content: &str, category: &str) -> String {
    // Use first line as title, truncated to reasonable length
    let first_line = content.lines().next().unwrap_or(content);
    let truncated = if first_line.len() > 50 {
        format!("{}...", &first_line[..47])
    } else {
        first_line.to_string()
    };

    format!("[{}] {}", category, truncated)
}

fn build_add_script(title: &str, body: &str, folder: Option<&str>) -> String {
    match folder {
        Some(f) => {
            let escaped_folder = escape_applescript_string(f);
            format!(
                r#"
tell application "Notes"
    set targetFolder to folder "{}" of default account
    make new note at targetFolder with properties {{name:"{}", body:"{}"}}
end tell
"#,
                escaped_folder, title, body
            )
        }
        None => {
            format!(
                r#"
tell application "Notes"
    make new note with properties {{name:"{}", body:"{}"}}
end tell
"#,
                title, body
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = NotesAdd;
        assert_eq!(tool.id(), "notes_add");
    }

    #[test]
    fn test_tool_name() {
        let tool = NotesAdd;
        assert_eq!(tool.name(), "Add Note");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = NotesAdd;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["content"].is_object());
        assert!(schema["properties"]["title"].is_object());
        assert!(schema["properties"]["category"].is_object());
        assert!(schema["properties"]["folder"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("content")));
    }

    #[test]
    fn test_execute_empty_content() {
        let tool = NotesAdd;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_execute_only_note_prefix() {
        let tool = NotesAdd;
        let result = tool.execute(json!({"content": "note:   "}));
        assert_eq!(result.status, "error");
    }

    #[test]
    fn test_generate_title_short() {
        let title = generate_title("Short content", "Work");
        assert_eq!(title, "[Work] Short content");
    }

    #[test]
    fn test_generate_title_long() {
        let long_content = "This is a very long content that should be truncated to fit within the title limit";
        let title = generate_title(long_content, "General");
        assert!(title.len() <= 60);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_build_script_default_folder() {
        let script = build_add_script("Test Note", "Test content", None);
        assert!(script.contains("Test Note"));
        assert!(script.contains("Test content"));
        assert!(!script.contains("folder"));
    }

    #[test]
    fn test_build_script_with_folder() {
        let script = build_add_script("Test Note", "Test content", Some("Work"));
        assert!(script.contains("Test Note"));
        assert!(script.contains("Test content"));
        assert!(script.contains("folder \"Work\""));
    }
}
