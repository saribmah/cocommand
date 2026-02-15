use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NoteSummaryPayload {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) preview: String,
    pub(super) path: String,
    pub(super) modified_at: Option<u64>,
    pub(super) size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotePayload {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) preview: String,
    pub(super) content: String,
    pub(super) path: String,
    pub(super) modified_at: Option<u64>,
    pub(super) size: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListNotesPayload {
    pub(super) root: String,
    pub(super) notes: Vec<NoteSummaryPayload>,
    pub(super) count: usize,
    pub(super) truncated: bool,
    pub(super) errors: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchNotesPayload {
    pub(super) query: String,
    pub(super) root: String,
    pub(super) notes: Vec<NoteSummaryPayload>,
    pub(super) count: usize,
    pub(super) truncated: bool,
    pub(super) scanned: usize,
    pub(super) errors: usize,
    pub(super) index_state: String,
    pub(super) index_scanned_files: usize,
    pub(super) index_scanned_dirs: usize,
    pub(super) index_started_at: Option<u64>,
    pub(super) index_last_update_at: Option<u64>,
    pub(super) index_finished_at: Option<u64>,
    pub(super) highlight_terms: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NoteIndexStatusPayload {
    pub(super) state: String,
    pub(super) root: String,
    pub(super) ignored_paths: Vec<String>,
    pub(super) indexed_entries: usize,
    pub(super) scanned_files: usize,
    pub(super) scanned_dirs: usize,
    pub(super) started_at: Option<u64>,
    pub(super) last_update_at: Option<u64>,
    pub(super) finished_at: Option<u64>,
    pub(super) errors: usize,
    pub(super) watcher_enabled: bool,
    pub(super) cache_path: String,
    pub(super) rescan_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_error: Option<String>,
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
