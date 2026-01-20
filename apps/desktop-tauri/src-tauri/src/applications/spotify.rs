//! Spotify application integration.
//!
//! Provides tools for controlling Spotify playback via AppleScript and Spotify Web API.
//!
//! # Submodules
//!
//! - `app`: SpotifyApp implementation
//! - `api`: Spotify Web API integration for searching and resolving URIs
//! - `open`: Open/activate Spotify app
//! - `play`: Play/resume tool
//! - `pause`: Pause tool
//! - `play_track`: Play specific track tool (demonstrates schema usage)
//! - `search_and_play`: Search and play by query (uses Web API)
//! - `play_artist`: Play music by artist (uses Web API)
//! - `play_album`: Play specific album (uses Web API)
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The SpotifyApp is registered in the applications registry and provides
//! tools: `spotify_open`, `spotify_play`, `spotify_pause`, `spotify_play_track`,
//! `spotify_search_and_play`, `spotify_play_artist`, and `spotify_play_album`.
//! These are mounted when the Spotify app is opened via `window.open`.
//!
//! # Environment Variables
//!
//! For search functionality, set the following environment variables:
//! - `SPOTIFY_CLIENT_ID`: Spotify API client ID
//! - `SPOTIFY_CLIENT_SECRET`: Spotify API client secret

pub mod api;
pub mod app;
pub mod open;
pub mod pause;
pub mod play;
pub mod play_album;
pub mod play_artist;
pub mod play_track;
pub mod script;
pub mod search_and_play;

// Re-export commonly used items
pub use app::{SpotifyApp, APP_ID};
pub use open::{SpotifyOpen, TOOL_ID as OPEN_TOOL_ID};
pub use pause::{SpotifyPause, TOOL_ID as PAUSE_TOOL_ID};
pub use play::{SpotifyPlay, TOOL_ID as PLAY_TOOL_ID};
pub use play_album::{SpotifyPlayAlbum, TOOL_ID as PLAY_ALBUM_TOOL_ID};
pub use play_artist::{SpotifyPlayArtist, TOOL_ID as PLAY_ARTIST_TOOL_ID};
pub use play_track::{SpotifyPlayTrack, TOOL_ID as PLAY_TRACK_TOOL_ID};
pub use search_and_play::{SpotifySearchAndPlay, TOOL_ID as SEARCH_AND_PLAY_TOOL_ID};
