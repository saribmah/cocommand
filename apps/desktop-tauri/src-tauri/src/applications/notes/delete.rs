//! Notes delete tool.
//!
//! This module provides the tool for deleting notes by name or content match.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the delete tool.
pub const TOOL_ID: &str = "notes_delete";

/// Tool for deleting a note.
pub struct NotesDelete;

impl Tool for NotesDelete {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Delete Note"
    }

    fn description(&self) -> &str {
        "Delete a note from Apple Notes by searching for it by title or content. Matches the first note that contains the query in its title or body."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find the note to delete. Matches against note title and body."
                },
                "exact_title": {
                    "type": "string",
                    "description": "Optional: Exact title of the note to delete. Takes precedence over query if provided."
                }
            },
            "required": ["query"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let exact_title = inputs
            .get("exact_title")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let query = inputs
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if exact_title.is_none() && query.is_empty() {
            return ToolResult::error("Query or exact_title is required to find the note to delete");
        }

        let script = if let Some(title) = exact_title {
            build_delete_by_title_script(title)
        } else {
            build_delete_by_query_script(query)
        };

        run_applescript(&script)
    }
}

fn build_delete_by_title_script(title: &str) -> String {
    let escaped = escape_applescript_string(title);
    format!(
        r#"
tell application "Notes"
    set foundNote to missing value
    set allNotes to every note of default account
    repeat with n in allNotes
        try
            if name of n is "{}" then
                set foundNote to n
                exit repeat
            end if
        end try
    end repeat

    if foundNote is missing value then
        return "NOT_FOUND: No note found with exact title '{}'"
    else
        set noteName to name of foundNote
        delete foundNote
        return "Deleted note: " & noteName
    end if
end tell
"#,
        escaped, escaped
    )
}

fn build_delete_by_query_script(query: &str) -> String {
    let escaped = escape_applescript_string(query);
    format!(
        r#"
tell application "Notes"
    set foundNote to missing value
    set searchQuery to "{}"
    set allNotes to every note of default account

    -- First try to match by title
    repeat with n in allNotes
        try
            if name of n contains searchQuery then
                set foundNote to n
                exit repeat
            end if
        end try
    end repeat

    -- If not found by title, try body
    if foundNote is missing value then
        repeat with n in allNotes
            try
                if body of n contains searchQuery then
                    set foundNote to n
                    exit repeat
                end if
            end try
        end repeat
    end if

    if foundNote is missing value then
        return "NOT_FOUND: No note found matching '{}'"
    else
        set noteName to name of foundNote
        delete foundNote
        return "Deleted note: " & noteName
    end if
end tell
"#,
        escaped, escaped
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = NotesDelete;
        assert_eq!(tool.id(), "notes_delete");
    }

    #[test]
    fn test_tool_name() {
        let tool = NotesDelete;
        assert_eq!(tool.name(), "Delete Note");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = NotesDelete;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["exact_title"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = NotesDelete;
        let result = tool.execute(json!({"query": ""}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_build_delete_by_title_script() {
        let script = build_delete_by_title_script("My Note");
        assert!(script.contains("name of n is \"My Note\""));
        assert!(script.contains("delete foundNote"));
    }

    #[test]
    fn test_build_delete_by_query_script() {
        let script = build_delete_by_query_script("meeting");
        assert!(script.contains("name of n contains searchQuery"));
        assert!(script.contains("body of n contains searchQuery"));
        assert!(script.contains("delete foundNote"));
    }
}
