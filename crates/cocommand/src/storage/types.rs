//! Domain structs for the storage layer.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::events::Event;

/// A stored event record with a sequence number for deterministic ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub seq: u64,
    pub event: Event,
}

impl EventRecord {
    pub fn id(&self) -> Uuid {
        self.event.id()
    }

    pub fn timestamp(&self) -> SystemTime {
        self.event.timestamp()
    }
}

/// A serializable workspace snapshot for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub session_id: String,
    pub captured_at: SystemTime,
    pub data: serde_json::Value,
}

/// A clipboard history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: Uuid,
    pub content: String,
    pub copied_at: SystemTime,
}
