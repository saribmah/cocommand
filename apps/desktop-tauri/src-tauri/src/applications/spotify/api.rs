//! Spotify Web API integration.
//!
//! This module provides functions for interacting with the Spotify Web API
//! to search for tracks, artists, albums, and playlists and retrieve their URIs.

use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Global cached token storage
static CACHED_TOKEN: OnceLock<Mutex<CachedToken>> = OnceLock::new();

struct CachedToken {
    access_token: Option<String>,
    expires_at: Option<Instant>,
}

fn get_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    })
}

fn get_cached_token_storage() -> &'static Mutex<CachedToken> {
    CACHED_TOKEN.get_or_init(|| {
        Mutex::new(CachedToken {
            access_token: None,
            expires_at: None,
        })
    })
}

/// Get a valid Spotify access token, using cache if available.
fn get_spotify_token() -> Option<String> {
    let client_id = env::var("SPOTIFY_CLIENT_ID").ok()?;
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET").ok()?;

    let cache = get_cached_token_storage();
    let mut cached = cache.lock().ok()?;

    // Check if we have a valid cached token
    if let (Some(token), Some(expires_at)) = (&cached.access_token, cached.expires_at) {
        if Instant::now() < expires_at {
            return Some(token.clone());
        }
    }

    // Request a new token using Client Credentials flow
    let client = get_client();
    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[("grant_type", "client_credentials")])
        .basic_auth(&client_id, Some(&client_secret))
        .send()
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        expires_in: u64,
    }

    let token_data: TokenResponse = response.json().ok()?;
    let expires_at = Instant::now() + Duration::from_secs(token_data.expires_in.saturating_sub(60));

    cached.access_token = Some(token_data.access_token.clone());
    cached.expires_at = Some(expires_at);

    Some(token_data.access_token)
}

/// Result of a Spotify search.
#[derive(Debug, Clone)]
pub struct SpotifySearchResult {
    pub uri: String,
    pub name: String,
    pub artist: Option<String>,
}

/// Search types supported by the Spotify API.
#[derive(Debug, Clone, Copy)]
pub enum SearchType {
    Track,
    Artist,
    Album,
    Playlist,
}

impl SearchType {
    fn as_str(&self) -> &'static str {
        match self {
            SearchType::Track => "track",
            SearchType::Artist => "artist",
            SearchType::Album => "album",
            SearchType::Playlist => "playlist",
        }
    }
}

// API response structures
#[derive(Deserialize)]
struct SearchResponse {
    tracks: Option<TracksResponse>,
    artists: Option<ArtistsResponse>,
    albums: Option<AlbumsResponse>,
    playlists: Option<PlaylistsResponse>,
}

#[derive(Deserialize)]
struct TracksResponse {
    items: Vec<TrackItem>,
}

#[derive(Deserialize)]
struct TrackItem {
    uri: String,
    name: String,
    artists: Vec<ArtistSimple>,
}

#[derive(Deserialize)]
struct ArtistSimple {
    name: String,
}

#[derive(Deserialize)]
struct ArtistsResponse {
    items: Vec<ArtistItem>,
}

#[derive(Deserialize)]
struct ArtistItem {
    uri: String,
    name: String,
}

#[derive(Deserialize)]
struct AlbumsResponse {
    items: Vec<AlbumItem>,
}

#[derive(Deserialize)]
struct AlbumItem {
    uri: String,
    name: String,
    artists: Vec<ArtistSimple>,
}

#[derive(Deserialize)]
struct PlaylistsResponse {
    items: Vec<PlaylistItem>,
}

#[derive(Deserialize)]
struct PlaylistItem {
    uri: String,
    name: String,
    owner: PlaylistOwner,
}

#[derive(Deserialize)]
struct PlaylistOwner {
    display_name: Option<String>,
}

/// Search Spotify for content and return the first result's URI.
///
/// # Arguments
/// * `query` - The search query
/// * `search_type` - The type of content to search for
///
/// # Returns
/// The first matching result, or None if no results found or API unavailable.
pub fn search_spotify(query: &str, search_type: SearchType) -> Option<SpotifySearchResult> {
    let token = get_spotify_token()?;

    let client = get_client();
    let type_str = search_type.as_str();

    let response = client
        .get("https://api.spotify.com/v1/search")
        .query(&[("q", query), ("type", type_str), ("limit", "1")])
        .bearer_auth(&token)
        .send()
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let search_result: SearchResponse = response.json().ok()?;

    match search_type {
        SearchType::Track => {
            let track = search_result.tracks?.items.into_iter().next()?;
            let artist = track.artists.first().map(|a| a.name.clone());
            Some(SpotifySearchResult {
                uri: track.uri,
                name: track.name,
                artist,
            })
        }
        SearchType::Artist => {
            let artist = search_result.artists?.items.into_iter().next()?;
            Some(SpotifySearchResult {
                uri: artist.uri,
                name: artist.name,
                artist: None,
            })
        }
        SearchType::Album => {
            let album = search_result.albums?.items.into_iter().next()?;
            let artist = album.artists.first().map(|a| a.name.clone());
            Some(SpotifySearchResult {
                uri: album.uri,
                name: album.name,
                artist,
            })
        }
        SearchType::Playlist => {
            let playlist = search_result.playlists?.items.into_iter().next()?;
            Some(SpotifySearchResult {
                uri: playlist.uri,
                name: playlist.name,
                artist: playlist.owner.display_name,
            })
        }
    }
}

/// Search for a track by name and optionally artist.
pub fn search_track(query: &str) -> Option<SpotifySearchResult> {
    search_spotify(query, SearchType::Track)
}

/// Search for an artist by name.
pub fn search_artist(artist_name: &str) -> Option<SpotifySearchResult> {
    search_spotify(artist_name, SearchType::Artist)
}

/// Search for an album by name and optionally artist.
pub fn search_album(album_name: &str, artist_name: Option<&str>) -> Option<SpotifySearchResult> {
    let query = match artist_name {
        Some(artist) => format!("album:{} artist:{}", album_name, artist),
        None => format!("album:{}", album_name),
    };
    search_spotify(&query, SearchType::Album)
}

/// Search for a playlist by name.
pub fn search_playlist(query: &str) -> Option<SpotifySearchResult> {
    search_spotify(query, SearchType::Playlist)
}

/// Check if Spotify API credentials are configured.
pub fn is_api_available() -> bool {
    env::var("SPOTIFY_CLIENT_ID").is_ok() && env::var("SPOTIFY_CLIENT_SECRET").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_type_as_str() {
        assert_eq!(SearchType::Track.as_str(), "track");
        assert_eq!(SearchType::Artist.as_str(), "artist");
        assert_eq!(SearchType::Album.as_str(), "album");
        assert_eq!(SearchType::Playlist.as_str(), "playlist");
    }
}
