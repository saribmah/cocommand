//! Storage module: traits, domain types, and implementations.

pub mod clipboard_store;
pub mod event_log;
pub mod kv_store;
pub mod memory;
pub mod snapshot_store;
pub mod traits;
pub mod types;

pub use clipboard_store::ClipboardStore;
pub use event_log::EventLog;
pub use kv_store::KvStore;
pub use memory::MemoryStorage;
pub use snapshot_store::SnapshotStore;
pub use traits::Storage;
pub use types::{event_summary, ClipboardEntry, EventRecord, WorkspaceSnapshot};
