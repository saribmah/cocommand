//! Spotify application integration.
//!
//! Provides tools for controlling Spotify playback via AppleScript.
//!
//! # Submodules
//!
//! - `app`: SpotifyApp implementation
//! - `play`: Play/resume tool
//! - `pause`: Pause tool
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The SpotifyApp is registered in the applications registry and provides
//! two tools: `spotify_play` and `spotify_pause`. These are mounted when
//! the Spotify app is opened via `window.open`.

pub mod app;
pub mod pause;
pub mod play;
pub mod script;

// Re-export commonly used items
pub use app::{SpotifyApp, APP_ID};
pub use pause::{SpotifyPause, TOOL_ID as PAUSE_TOOL_ID};
pub use play::{SpotifyPlay, TOOL_ID as PLAY_TOOL_ID};
