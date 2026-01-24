//! Bounded clipboard history trait and in-memory implementation.

use super::types::ClipboardEntry;

const DEFAULT_CLIPBOARD_MAX: usize = 50;

/// Bounded clipboard history with deduplication.
pub trait ClipboardStore: Send + Sync {
    /// Push a new entry. Consecutive entries with identical content are deduplicated.
    fn push(&mut self, entry: ClipboardEntry);
    /// List entries in most-recent-first order, up to `limit`.
    fn list(&self, limit: usize) -> Vec<ClipboardEntry>;
    /// Get the most recent entry.
    fn latest(&self) -> Option<ClipboardEntry>;
    /// Number of stored entries.
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

// --- Memory Implementation ---

#[derive(Debug)]
pub(crate) struct MemoryClipboardStore {
    entries: Vec<ClipboardEntry>,
    max_entries: usize,
}

impl Default for MemoryClipboardStore {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: DEFAULT_CLIPBOARD_MAX,
        }
    }
}

impl ClipboardStore for MemoryClipboardStore {
    fn push(&mut self, entry: ClipboardEntry) {
        // Deduplicate consecutive identical content.
        if let Some(last) = self.entries.last() {
            if last.content == entry.content {
                return;
            }
        }
        self.entries.push(entry);
        // Enforce bound — drop oldest when over capacity.
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    fn list(&self, limit: usize) -> Vec<ClipboardEntry> {
        self.entries.iter().rev().take(limit).cloned().collect()
    }

    fn latest(&self) -> Option<ClipboardEntry> {
        self.entries.last().cloned()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[test]
    fn starts_empty() {
        let clip = MemoryClipboardStore::default();
        assert!(clip.is_empty());
        assert_eq!(clip.len(), 0);
        assert!(clip.latest().is_none());
    }

    #[test]
    fn push_and_latest() {
        let mut clip = MemoryClipboardStore::default();
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "hello".to_string(),
            copied_at: SystemTime::now(),
        });

        assert_eq!(clip.len(), 1);
        assert_eq!(clip.latest().unwrap().content, "hello");
    }

    #[test]
    fn list_most_recent_first() {
        let mut clip = MemoryClipboardStore::default();
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "first".to_string(),
            copied_at: SystemTime::now(),
        });
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "second".to_string(),
            copied_at: SystemTime::now(),
        });
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "third".to_string(),
            copied_at: SystemTime::now(),
        });

        let listed = clip.list(10);
        assert_eq!(listed[0].content, "third");
        assert_eq!(listed[1].content, "second");
        assert_eq!(listed[2].content, "first");
    }

    #[test]
    fn list_respects_limit() {
        let mut clip = MemoryClipboardStore::default();
        for i in 0..5 {
            clip.push(ClipboardEntry {
                id: Uuid::new_v4(),
                content: format!("item-{i}"),
                copied_at: SystemTime::now(),
            });
        }
        assert_eq!(clip.list(2).len(), 2);
    }

    #[test]
    fn deduplicates_consecutive() {
        let mut clip = MemoryClipboardStore::default();
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "same".to_string(),
            copied_at: SystemTime::now(),
        });
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "same".to_string(),
            copied_at: SystemTime::now(),
        });
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "different".to_string(),
            copied_at: SystemTime::now(),
        });
        clip.push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "same".to_string(),
            copied_at: SystemTime::now(),
        });

        assert_eq!(clip.len(), 3);
    }

    #[test]
    fn bounds_at_max() {
        let mut clip = MemoryClipboardStore::default();
        for i in 0..(DEFAULT_CLIPBOARD_MAX + 10) {
            clip.push(ClipboardEntry {
                id: Uuid::new_v4(),
                content: format!("item-{i}"),
                copied_at: SystemTime::now(),
            });
        }

        assert_eq!(clip.len(), DEFAULT_CLIPBOARD_MAX);
        let latest = clip.latest().unwrap();
        assert_eq!(latest.content, format!("item-{}", DEFAULT_CLIPBOARD_MAX + 9));
    }
}
