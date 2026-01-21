//! Application registry for managing and executing application tools.
//!
//! This module provides the central registry for all applications and their tools.
//! It handles:
//! - Listing all available applications
//! - Looking up applications by ID
//! - Executing tools by ID
//!
//! # Architecture
//!
//! The registry follows opencode's pattern of:
//! - Apps as capability packs that provide tool bundles
//! - Fresh instantiation per call (no caching)
//! - Pattern-matched dispatch for tool execution

use serde_json::Value;

use super::calendar::{
    self, ADD_TOOL_ID as CALENDAR_ADD_TOOL_ID, CANCEL_TOOL_ID as CALENDAR_CANCEL_TOOL_ID,
    LIST_UPCOMING_TOOL_ID as CALENDAR_LIST_UPCOMING_TOOL_ID, OPEN_TOOL_ID as CALENDAR_OPEN_TOOL_ID,
    RESCHEDULE_TOOL_ID as CALENDAR_RESCHEDULE_TOOL_ID,
};
use super::notes::{
    self, ADD_TOOL_ID as NOTES_ADD_TOOL_ID, DELETE_TOOL_ID as NOTES_DELETE_TOOL_ID,
    LIST_TOOL_ID as NOTES_LIST_TOOL_ID, OPEN_TOOL_ID as NOTES_OPEN_TOOL_ID,
    SUMMARIZE_TOOL_ID as NOTES_SUMMARIZE_TOOL_ID,
};
use super::reminders::{
    self, ADD_TOOL_ID as REMINDERS_ADD_TOOL_ID, CANCEL_TOOL_ID as REMINDERS_CANCEL_TOOL_ID,
    COMPLETE_TOOL_ID as REMINDERS_COMPLETE_TOOL_ID,
    LIST_UPCOMING_TOOL_ID as REMINDERS_LIST_UPCOMING_TOOL_ID,
    OPEN_TOOL_ID as REMINDERS_OPEN_TOOL_ID, RESCHEDULE_TOOL_ID as REMINDERS_RESCHEDULE_TOOL_ID,
};
use super::spotify::{
    self, OPEN_TOOL_ID as SPOTIFY_OPEN_TOOL_ID, PAUSE_TOOL_ID, PLAY_ALBUM_TOOL_ID,
    PLAY_ARTIST_TOOL_ID, PLAY_TOOL_ID, PLAY_TRACK_TOOL_ID, SEARCH_AND_PLAY_TOOL_ID,
};
use super::types::{Application, ApplicationDefinition, Tool, ToolDefinition, ToolResult};

/// Get all registered applications.
///
/// Returns a list of all applications with their tools.
/// Applications are instantiated fresh each call.
pub fn all_apps() -> Vec<ApplicationDefinition> {
    let apps: Vec<Box<dyn Application>> = vec![
        Box::new(spotify::SpotifyApp::default()),
        Box::new(reminders::RemindersApp::default()),
        Box::new(notes::NotesApp::default()),
        Box::new(calendar::CalendarApp::default()),
    ];

    apps.into_iter()
        .map(|app| ApplicationDefinition {
            id: app.id().to_string(),
            name: app.name().to_string(),
            description: app.description().to_string(),
            tools: app.tools(),
        })
        .collect()
}

/// Get all tools from all applications.
///
/// Returns a flat list of all tool definitions.
pub fn all_tools() -> Vec<ToolDefinition> {
    all_apps()
        .into_iter()
        .flat_map(|app| app.tools)
        .collect()
}

/// Find an application by its ID.
///
/// Returns the application definition if found.
pub fn app_by_id(app_id: &str) -> Option<ApplicationDefinition> {
    all_apps().into_iter().find(|app| app.id == app_id)
}

/// Get tools for a specific application by ID.
///
/// Returns the tool definitions for the application if found.
pub fn tools_for_app(app_id: &str) -> Option<Vec<ToolDefinition>> {
    app_by_id(app_id).map(|app| app.tools)
}

/// Check if a tool belongs to an application.
///
/// Returns true if the tool ID starts with the app ID prefix.
pub fn tool_belongs_to_app(tool_id: &str, app_id: &str) -> bool {
    tool_id.starts_with(&format!("{}_", app_id))
}

/// Execute a tool by its ID.
///
/// Routes to the appropriate tool implementation based on the tool ID.
/// Returns None if the tool is not found.
pub fn execute_tool(tool_id: &str, inputs: Value) -> Option<ToolResult> {
    match tool_id {
        // Spotify tools
        id if id == SPOTIFY_OPEN_TOOL_ID => Some(spotify::SpotifyOpen.execute(inputs)),
        id if id == PLAY_TOOL_ID => Some(spotify::SpotifyPlay.execute(inputs)),
        id if id == PAUSE_TOOL_ID => Some(spotify::SpotifyPause.execute(inputs)),
        id if id == SEARCH_AND_PLAY_TOOL_ID => Some(spotify::SpotifySearchAndPlay.execute(inputs)),
        id if id == PLAY_ARTIST_TOOL_ID => Some(spotify::SpotifyPlayArtist.execute(inputs)),
        id if id == PLAY_ALBUM_TOOL_ID => Some(spotify::SpotifyPlayAlbum.execute(inputs)),
        id if id == PLAY_TRACK_TOOL_ID => Some(spotify::SpotifyPlayTrack.execute(inputs)),
        // Reminders tools
        id if id == REMINDERS_OPEN_TOOL_ID => Some(reminders::RemindersOpen.execute(inputs)),
        id if id == REMINDERS_ADD_TOOL_ID => Some(reminders::RemindersAdd.execute(inputs)),
        id if id == REMINDERS_LIST_UPCOMING_TOOL_ID => {
            Some(reminders::RemindersListUpcoming.execute(inputs))
        }
        id if id == REMINDERS_CANCEL_TOOL_ID => Some(reminders::RemindersCancel.execute(inputs)),
        id if id == REMINDERS_RESCHEDULE_TOOL_ID => {
            Some(reminders::RemindersReschedule.execute(inputs))
        }
        id if id == REMINDERS_COMPLETE_TOOL_ID => Some(reminders::RemindersComplete.execute(inputs)),
        // Notes tools
        id if id == NOTES_OPEN_TOOL_ID => Some(notes::NotesOpen.execute(inputs)),
        id if id == NOTES_ADD_TOOL_ID => Some(notes::NotesAdd.execute(inputs)),
        id if id == NOTES_LIST_TOOL_ID => Some(notes::NotesList.execute(inputs)),
        id if id == NOTES_DELETE_TOOL_ID => Some(notes::NotesDelete.execute(inputs)),
        id if id == NOTES_SUMMARIZE_TOOL_ID => Some(notes::NotesSummarize.execute(inputs)),
        // Calendar tools
        id if id == CALENDAR_OPEN_TOOL_ID => Some(calendar::CalendarOpen.execute(inputs)),
        id if id == CALENDAR_ADD_TOOL_ID => Some(calendar::CalendarAdd.execute(inputs)),
        id if id == CALENDAR_LIST_UPCOMING_TOOL_ID => {
            Some(calendar::CalendarListUpcoming.execute(inputs))
        }
        id if id == CALENDAR_CANCEL_TOOL_ID => Some(calendar::CalendarCancel.execute(inputs)),
        id if id == CALENDAR_RESCHEDULE_TOOL_ID => {
            Some(calendar::CalendarReschedule.execute(inputs))
        }
        _ => None,
    }
}

/// Get the app ID from a tool ID.
///
/// Extracts the app prefix from tool IDs that follow the `appid_action` pattern.
pub fn app_id_from_tool(tool_id: &str) -> Option<&str> {
    tool_id.split('_').next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_apps_returns_spotify_reminders_notes_and_calendar() {
        let apps = all_apps();
        assert!(!apps.is_empty());
        assert!(apps.iter().any(|a| a.id == "spotify"));
        assert!(apps.iter().any(|a| a.id == "reminders"));
        assert!(apps.iter().any(|a| a.id == "notes"));
        assert!(apps.iter().any(|a| a.id == "calendar"));
    }

    #[test]
    fn test_app_by_id_found() {
        let app = app_by_id("spotify");
        assert!(app.is_some());
        assert_eq!(app.unwrap().id, "spotify");

        let app = app_by_id("reminders");
        assert!(app.is_some());
        assert_eq!(app.unwrap().id, "reminders");

        let app = app_by_id("notes");
        assert!(app.is_some());
        assert_eq!(app.unwrap().id, "notes");

        let app = app_by_id("calendar");
        assert!(app.is_some());
        assert_eq!(app.unwrap().id, "calendar");
    }

    #[test]
    fn test_app_by_id_not_found() {
        let app = app_by_id("nonexistent");
        assert!(app.is_none());
    }

    #[test]
    fn test_all_tools() {
        let tools = all_tools();
        // Spotify tools
        assert!(tools.iter().any(|t| t.id == "spotify_play"));
        assert!(tools.iter().any(|t| t.id == "spotify_pause"));
        // Reminders tools
        assert!(tools.iter().any(|t| t.id == "reminders_add"));
        assert!(tools.iter().any(|t| t.id == "reminders_complete"));
        // Notes tools
        assert!(tools.iter().any(|t| t.id == "notes_add"));
        assert!(tools.iter().any(|t| t.id == "notes_list"));
        // Calendar tools
        assert!(tools.iter().any(|t| t.id == "calendar_add"));
        assert!(tools.iter().any(|t| t.id == "calendar_list_upcoming"));
    }

    #[test]
    fn test_execute_tool_unknown() {
        let result = execute_tool("unknown_tool", serde_json::json!({}));
        assert!(result.is_none());
    }

    #[test]
    fn test_tools_for_spotify() {
        let tools = tools_for_app("spotify");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn test_tools_for_reminders() {
        let tools = tools_for_app("reminders");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn test_play_track_tool_has_schema() {
        let tools = all_tools();
        let play_track = tools.iter().find(|t| t.id == "spotify_play_track").unwrap();
        assert!(play_track.schema.is_some());
    }

    #[test]
    fn test_reminders_add_tool_has_schema() {
        let tools = all_tools();
        let add_tool = tools.iter().find(|t| t.id == "reminders_add").unwrap();
        assert!(add_tool.schema.is_some());
    }

    #[test]
    fn test_all_spotify_tools_present() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.id == "spotify_open"));
        assert!(tools.iter().any(|t| t.id == "spotify_play"));
        assert!(tools.iter().any(|t| t.id == "spotify_pause"));
        assert!(tools.iter().any(|t| t.id == "spotify_search_and_play"));
        assert!(tools.iter().any(|t| t.id == "spotify_play_artist"));
        assert!(tools.iter().any(|t| t.id == "spotify_play_album"));
        assert!(tools.iter().any(|t| t.id == "spotify_play_track"));
    }

    #[test]
    fn test_all_reminders_tools_present() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.id == "reminders_open"));
        assert!(tools.iter().any(|t| t.id == "reminders_add"));
        assert!(tools.iter().any(|t| t.id == "reminders_list_upcoming"));
        assert!(tools.iter().any(|t| t.id == "reminders_cancel"));
        assert!(tools.iter().any(|t| t.id == "reminders_reschedule"));
        assert!(tools.iter().any(|t| t.id == "reminders_complete"));
    }

    #[test]
    fn test_all_notes_tools_present() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.id == "notes_open"));
        assert!(tools.iter().any(|t| t.id == "notes_add"));
        assert!(tools.iter().any(|t| t.id == "notes_list"));
        assert!(tools.iter().any(|t| t.id == "notes_delete"));
        assert!(tools.iter().any(|t| t.id == "notes_summarize"));
    }

    #[test]
    fn test_all_calendar_tools_present() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.id == "calendar_open"));
        assert!(tools.iter().any(|t| t.id == "calendar_add"));
        assert!(tools.iter().any(|t| t.id == "calendar_list_upcoming"));
        assert!(tools.iter().any(|t| t.id == "calendar_cancel"));
        assert!(tools.iter().any(|t| t.id == "calendar_reschedule"));
    }

    #[test]
    fn test_tools_for_notes() {
        let tools = tools_for_app("notes");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn test_tools_for_calendar() {
        let tools = tools_for_app("calendar");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn test_notes_add_tool_has_schema() {
        let tools = all_tools();
        let add_tool = tools.iter().find(|t| t.id == "notes_add").unwrap();
        assert!(add_tool.schema.is_some());
    }

    #[test]
    fn test_calendar_add_tool_has_schema() {
        let tools = all_tools();
        let add_tool = tools.iter().find(|t| t.id == "calendar_add").unwrap();
        assert!(add_tool.schema.is_some());
    }

    #[test]
    fn test_tool_belongs_to_app() {
        assert!(tool_belongs_to_app("spotify_play", "spotify"));
        assert!(tool_belongs_to_app("spotify_pause", "spotify"));
        assert!(!tool_belongs_to_app("spotify_play", "apple_music"));
        assert!(tool_belongs_to_app("reminders_add", "reminders"));
        assert!(tool_belongs_to_app("reminders_complete", "reminders"));
        assert!(!tool_belongs_to_app("reminders_add", "spotify"));
        assert!(tool_belongs_to_app("calendar_add", "calendar"));
        assert!(tool_belongs_to_app("calendar_reschedule", "calendar"));
        assert!(!tool_belongs_to_app("calendar_add", "reminders"));
    }

    #[test]
    fn test_app_id_from_tool() {
        assert_eq!(app_id_from_tool("spotify_play"), Some("spotify"));
        assert_eq!(app_id_from_tool("reminders_add"), Some("reminders"));
        assert_eq!(app_id_from_tool("calendar_add"), Some("calendar"));
        assert_eq!(app_id_from_tool("unknown"), Some("unknown"));
    }
}
