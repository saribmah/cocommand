//! Storage facade trait.

use super::clipboard_store::ClipboardStore;
use super::event_log::EventLog;
use super::kv_store::KvStore;
use super::snapshot_store::SnapshotStore;

/// Facade trait providing access to all specialized sub-stores.
pub trait Storage: Send + Sync {
    fn event_log(&self) -> &dyn EventLog;
    fn event_log_mut(&mut self) -> &mut dyn EventLog;
    fn snapshots(&self) -> &dyn SnapshotStore;
    fn snapshots_mut(&mut self) -> &mut dyn SnapshotStore;
    fn kv(&self) -> &dyn KvStore;
    fn kv_mut(&mut self) -> &mut dyn KvStore;
    fn clipboard(&self) -> &dyn ClipboardStore;
    fn clipboard_mut(&mut self) -> &mut dyn ClipboardStore;

    /// Split mutable access to event log and clipboard store simultaneously.
    /// Required for tool execution where handlers may need both.
    fn split_event_clipboard_mut(&mut self) -> (&mut dyn EventLog, &mut dyn ClipboardStore);
}
