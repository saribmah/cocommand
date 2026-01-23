//! Notes summarize tool.
//!
//! This module provides the tool for getting all notes content for summarization.
//! The actual summarization is performed by the agent using LLM.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the summarize tool.
pub const TOOL_ID: &str = "notes_summarize";

/// Tool for getting notes content for summarization.
pub struct NotesSummarize;

impl Tool for NotesSummarize {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Summarize Notes"
    }

    fn description(&self) -> &str {
        "Get all notes content for analysis and summarization. Returns notes grouped by category with their content. Use this when the user asks to analyze or summarize their notes."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Optional: Maximum number of notes to include. Defaults to 50."
                },
                "category": {
                    "type": "string",
                    "description": "Optional: Filter by category to summarize only notes in that category."
                },
                "folder": {
                    "type": "string",
                    "description": "Optional: Filter by folder to summarize only notes in that folder."
                }
            }
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let limit = inputs
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(50) as usize;

        let category = inputs
            .get("category")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let folder = inputs
            .get("folder")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let script = build_summarize_script(limit, category, folder);
        let result = run_applescript(&script);

        if result.status == "ok" {
            if result.message.trim().is_empty() {
                ToolResult::ok("No notes found to summarize.")
            } else {
                // Format the response with instructions for the agent
                let formatted = format!(
                    "Here are your notes for analysis:\n\n{}\n\nPlease analyze these notes and provide a summary.",
                    result.message
                );
                ToolResult::ok(formatted)
            }
        } else {
            result
        }
    }
}

fn build_summarize_script(limit: usize, category: Option<&str>, folder: Option<&str>) -> String {
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

    format!(
        r#"
tell application "Notes"
    set outputList to {{}}
    set counter to 0
    set noteList to ({})

    repeat with n in noteList
        if counter >= {} then exit repeat

        try
            set matchesCategory to {}

            if matchesCategory then
                set noteName to name of n
                set noteBody to body of n

                -- Extract plain text from body (remove HTML tags)
                set plainBody to noteBody
                try
                    -- Simple HTML tag removal
                    set tid to AppleScript's text item delimiters
                    set AppleScript's text item delimiters to "<"
                    set bodyParts to text items of plainBody
                    set AppleScript's text item delimiters to ">"
                    set cleanParts to {{}}
                    repeat with part in bodyParts
                        set textItems to text items of part
                        if (count of textItems) > 1 then
                            set end of cleanParts to item 2 of textItems
                        else
                            set end of cleanParts to part
                        end if
                    end repeat
                    set AppleScript's text item delimiters to ""
                    set plainBody to cleanParts as text
                    set AppleScript's text item delimiters to tid
                end try

                -- Extract category from body if present
                set categoryTag to "General"
                try
                    if plainBody contains "[" and plainBody contains "]" then
                        set startPos to offset of "[" in plainBody
                        set endPos to offset of "]" in plainBody
                        if endPos > startPos then
                            set categoryTag to text (startPos + 1) thru (endPos - 1) of plainBody
                        end if
                    end if
                end try

                -- Format: Title [Category]: Content (truncated)
                set contentPreview to plainBody
                if (length of contentPreview) > 200 then
                    set contentPreview to text 1 thru 200 of contentPreview & "..."
                end if

                set end of outputList to "---"
                set end of outputList to "Title: " & noteName
                set end of outputList to "Category: " & categoryTag
                set end of outputList to "Content: " & contentPreview
                set end of outputList to ""

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
        folder_clause, limit, category_filter
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = NotesSummarize;
        assert_eq!(tool.id(), "notes_summarize");
    }

    #[test]
    fn test_tool_name() {
        let tool = NotesSummarize;
        assert_eq!(tool.name(), "Summarize Notes");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = NotesSummarize;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["properties"]["category"].is_object());
        assert!(schema["properties"]["folder"].is_object());
    }

    #[test]
    fn test_build_summarize_script_defaults() {
        let script = build_summarize_script(50, None, None);
        assert!(script.contains(">= 50"));
        assert!(script.contains("every note of default account"));
        assert!(script.contains("true")); // No category filter
    }

    #[test]
    fn test_build_summarize_script_with_category() {
        let script = build_summarize_script(20, Some("Work"), None);
        assert!(script.contains("[Work]"));
    }

    #[test]
    fn test_build_summarize_script_with_folder() {
        let script = build_summarize_script(20, None, Some("Projects"));
        assert!(script.contains("folder \"Projects\""));
    }
}
