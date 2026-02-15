use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummaryPayload {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub path: String,
    pub modified_at: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePayload {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub content: String,
    pub path: String,
    pub modified_at: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotesPayload {
    pub root: String,
    pub notes: Vec<NoteSummaryPayload>,
    pub count: usize,
    pub truncated: bool,
    pub errors: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchNotesPayload {
    pub query: String,
    pub root: String,
    pub notes: Vec<NoteSummaryPayload>,
    pub count: usize,
    pub truncated: bool,
    pub scanned: usize,
    pub errors: usize,
    pub index_state: String,
    pub index_scanned_files: usize,
    pub index_scanned_dirs: usize,
    pub index_started_at: Option<u64>,
    pub index_last_update_at: Option<u64>,
    pub index_finished_at: Option<u64>,
    pub highlight_terms: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteIndexStatusPayload {
    pub state: String,
    pub root: String,
    pub ignored_paths: Vec<String>,
    pub indexed_entries: usize,
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub started_at: Option<u64>,
    pub last_update_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub errors: usize,
    pub watcher_enabled: bool,
    pub cache_path: String,
    pub rescan_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl From<filesystem::IndexStatus> for NoteIndexStatusPayload {
    fn from(status: filesystem::IndexStatus) -> Self {
        Self {
            state: status.state,
            root: status.root,
            ignored_paths: status.ignored_paths,
            indexed_entries: status.indexed_entries,
            scanned_files: status.scanned_files,
            scanned_dirs: status.scanned_dirs,
            started_at: status.started_at,
            last_update_at: status.last_update_at,
            finished_at: status.finished_at,
            errors: status.errors,
            watcher_enabled: status.watcher_enabled,
            cache_path: status.cache_path,
            rescan_count: status.rescan_count,
            last_error: status.last_error,
        }
    }
}
