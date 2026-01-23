//! Notes list tool.
//!
//! This module provides the tool for listing notes with optional filtering.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the list tool.
pub const TOOL_ID: &str = "notes_list";

/// Tool for listing notes.
pub struct NotesList;

impl Tool for NotesList {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "List Notes"
    }

    fn description(&self) -> &str {
        "List notes from Apple Notes. Can filter by category, folder, or search query. Returns note titles and categories."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Optional: Maximum number of notes to return. Defaults to 20."
                },
                "folder": {
                    "type": "string",
                    "description": "Optional: Name of a specific Notes folder to list from."
                },
                "category": {
                    "type": "string",
                    "description": "Optional: Filter by category (e.g., 'Work', 'Personal', 'Ideas'). Matches notes with [Category] in title or body."
                },
                "search": {
                    "type": "string",
                    "description": "Optional: Search query to filter notes by title or content."
                }
            }
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let limit = inputs
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as usize;

        let folder = inputs
            .get("folder")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let category = inputs
            .get("category")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let search = inputs
            .get("search")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let script = build_list_script(limit, folder, category, search);
        let result = run_applescript(&script);

        if result.status == "ok" {
            if result.message.trim().is_empty() {
                ToolResult::ok("No notes found.")
            } else {
                result
            }
        } else {
            result
        }
    }
}

fn build_list_script(
    limit: usize,
    folder: Option<&str>,
    category: Option<&str>,
    search: Option<&str>,
) -> String {
    let folder_clause = match folder {
        Some(f) => {
            let escaped = escape_applescript_string(f);
            format!("every note of folder \"{}\" of default account", escaped)
        }
        None => "every note of default account".to_string(),
    };

    let category_filter = match category {
        Some(c) => {
            let escaped = escape_applescript_string(c);
            format!(
                "(name of n contains \"[{0}]\" or body of n contains \"[{0}]\")",
                escaped
            )
        }
        None => "true".to_string(),
    };

    let search_filter = match search {
        Some(s) => {
            let escaped = escape_applescript_string(s);
            format!(
                "(name of n contains \"{0}\" or body of n contains \"{0}\")",
                escaped
            )
        }
        None => "true".to_string(),
    };

    format!(
        r#"
tell application "Notes"
    set outputList to {{}}
    set counter to 0
    set noteList to ({})

    repeat with n in noteList
        if counter >= {} then exit repeat

        try
            -- Apply filters
            set matchesCategory to {}
            set matchesSearch to {}

            if matchesCategory and matchesSearch then
                set noteName to name of n
                set noteBody to body of n

                -- Extract category from body if present (format: [Category] content)
                set categoryTag to ""
                try
                    if noteBody contains "[" and noteBody contains "]" then
                        set startPos to offset of "[" in noteBody
                        set endPos to offset of "]" in noteBody
                        if endPos > startPos then
                            set categoryTag to text (startPos + 1) thru (endPos - 1) of noteBody
                        end if
                    end if
                end try

                -- Format output
                if categoryTag is not "" then
                    set end of outputList to "- " & noteName & " [" & categoryTag & "]"
                else
                    set end of outputList to "- " & noteName
                end if

                set counter to counter + 1
            end if
        end try
    end repeat

    if (count of outputList) = 0 then
        return ""
    else
        set AppleScript's text item delimiters to linefeed
        return outputList as text
    end if
end tell
"#,
        folder_clause, limit, category_filter, search_filter
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = NotesList;
        assert_eq!(tool.id(), "notes_list");
    }

    #[test]
    fn test_tool_name() {
        let tool = NotesList;
        assert_eq!(tool.name(), "List Notes");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = NotesList;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["properties"]["folder"].is_object());
        assert!(schema["properties"]["category"].is_object());
        assert!(schema["properties"]["search"].is_object());
    }

    #[test]
    fn test_build_list_script_defaults() {
        let script = build_list_script(20, None, None, None);
        assert!(script.contains(">= 20"));
        assert!(script.contains("every note of default account"));
        assert!(script.contains("true")); // No filters
    }

    #[test]
    fn test_build_list_script_with_folder() {
        let script = build_list_script(10, Some("Work"), None, None);
        assert!(script.contains("folder \"Work\""));
    }

    #[test]
    fn test_build_list_script_with_category() {
        let script = build_list_script(10, None, Some("Ideas"), None);
        assert!(script.contains("[Ideas]"));
    }

    #[test]
    fn test_build_list_script_with_search() {
        let script = build_list_script(10, None, None, Some("meeting"));
        assert!(script.contains("meeting"));
    }
}
