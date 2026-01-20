//! Spotify application integration.
//!
//! Provides tools for controlling Spotify playback via AppleScript.
//!
//! # Submodules
//!
//! - `app`: SpotifyApp implementation
//! - `play`: Play/resume tool
//! - `pause`: Pause tool
//! - `play_track`: Play specific track tool (demonstrates schema usage)
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The SpotifyApp is registered in the applications registry and provides
//! tools: `spotify_play`, `spotify_pause`, and `spotify_play_track`.
//! These are mounted when the Spotify app is opened via `window.open`.

pub mod app;
pub mod pause;
pub mod play;
pub mod play_track;
pub mod script;

// Re-export commonly used items
pub use app::{SpotifyApp, APP_ID};
pub use pause::{SpotifyPause, TOOL_ID as PAUSE_TOOL_ID};
pub use play::{SpotifyPlay, TOOL_ID as PLAY_TOOL_ID};
pub use play_track::{SpotifyPlayTrack, TOOL_ID as PLAY_TRACK_TOOL_ID};
