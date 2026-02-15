//! Notes API endpoints.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::error::CoreError;
use crate::extension::builtin::note::ops::{
    create_note, delete_note, list_notes, notes_root, read_note, update_note,
};
use crate::extension::builtin::note::types::{ListNotesPayload, NotePayload};
use crate::server::ServerState;

/// Request payload for creating a note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    /// Optional title for the note.
    pub title: Option<String>,
    /// Optional initial content.
    pub content: Option<String>,
    /// Optional folder path under notes/.
    pub folder: Option<String>,
}

/// Request payload for listing notes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotesRequest {
    /// Maximum number of notes to return. Defaults to 50, max 500.
    pub limit: Option<usize>,
}

/// Request payload for getting a note by ID.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNoteRequest {
    /// The note ID (e.g., "folder/note-name").
    pub id: String,
}

/// Request payload for updating a note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    /// The note ID (e.g., "folder/note-name").
    pub id: String,
    /// New content for the note.
    pub content: String,
}

/// Request payload for deleting a note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNoteRequest {
    /// The note ID (e.g., "folder/note-name").
    pub id: String,
}

/// Response for delete operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNoteResponse {
    pub status: String,
    pub deleted: bool,
}

/// POST /extension/notes/create
///
/// Creates a new note.
pub(crate) async fn create(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<Json<NotePayload>, (StatusCode, String)> {
    let workspace_dir = state.workspace.workspace_dir.clone();
    let notes_root = notes_root(&workspace_dir);

    let title = payload.title;
    let content = payload.content;
    let folder = payload.folder;

    let result = tokio::task::spawn_blocking(move || create_note(notes_root, title, content, folder))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task failed: {e}"),
            )
        })?
        .map_err(map_core_error)?;

    Ok(Json(result))
}

/// POST /extension/notes/list
///
/// Lists all notes, sorted by last modified time.
pub(crate) async fn list(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<ListNotesRequest>,
) -> Result<Json<ListNotesPayload>, (StatusCode, String)> {
    let workspace_dir = state.workspace.workspace_dir.clone();
    let notes_root = notes_root(&workspace_dir);
    let limit = payload.limit.unwrap_or(50).clamp(1, 500);

    let result = tokio::task::spawn_blocking(move || list_notes(notes_root, limit))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task failed: {e}"),
            )
        })?
        .map_err(map_core_error)?;

    Ok(Json(result))
}

/// POST /extension/notes/get
///
/// Reads a note by ID.
pub(crate) async fn get(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<GetNoteRequest>,
) -> Result<Json<NotePayload>, (StatusCode, String)> {
    let workspace_dir = state.workspace.workspace_dir.clone();
    let notes_root = notes_root(&workspace_dir);
    let id = payload.id;

    let result = tokio::task::spawn_blocking(move || read_note(notes_root, id))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task failed: {e}"),
            )
        })?
        .map_err(map_core_error)?;

    Ok(Json(result))
}

/// POST /extension/notes/update
///
/// Updates a note's content by ID.
pub(crate) async fn update(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateNoteRequest>,
) -> Result<Json<NotePayload>, (StatusCode, String)> {
    let workspace_dir = state.workspace.workspace_dir.clone();
    let notes_root = notes_root(&workspace_dir);
    let id = payload.id;
    let content = payload.content;

    let result = tokio::task::spawn_blocking(move || update_note(notes_root, id, content))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task failed: {e}"),
            )
        })?
        .map_err(map_core_error)?;

    Ok(Json(result))
}

/// POST /extension/notes/delete
///
/// Deletes a note by ID.
pub(crate) async fn delete(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<DeleteNoteRequest>,
) -> Result<Json<DeleteNoteResponse>, (StatusCode, String)> {
    let workspace_dir = state.workspace.workspace_dir.clone();
    let notes_root = notes_root(&workspace_dir);
    let id = payload.id;

    let deleted = tokio::task::spawn_blocking(move || delete_note(notes_root, id))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task failed: {e}"),
            )
        })?
        .map_err(map_core_error)?;

    Ok(Json(DeleteNoteResponse {
        status: "ok".to_string(),
        deleted,
    }))
}

fn map_core_error(error: CoreError) -> (StatusCode, String) {
    match &error {
        CoreError::InvalidInput(msg) => {
            // Check if it's a "not found" type error
            if msg.contains("not found") {
                (StatusCode::NOT_FOUND, error.to_string())
            } else {
                (StatusCode::BAD_REQUEST, error.to_string())
            }
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}
