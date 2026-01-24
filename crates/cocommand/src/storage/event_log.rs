//! Append-only event log trait and in-memory implementation.

use crate::events::Event;

use super::types::EventRecord;

/// Append-only event log with deterministic sequencing.
pub trait EventLog: Send + Sync {
    /// Append an event and return the created record with its sequence number.
    fn append(&mut self, event: Event) -> EventRecord;
    /// Total number of records.
    fn len(&self) -> usize;
    /// Whether the log is empty.
    fn is_empty(&self) -> bool;
    /// Return the last `limit` records (oldest first within the slice).
    fn tail(&self, limit: usize) -> Vec<EventRecord>;
    /// Return all records with seq > `seq`.
    fn since(&self, seq: u64) -> Vec<EventRecord>;
}

// --- Memory Implementation ---

#[derive(Debug, Default)]
pub(crate) struct MemoryEventLog {
    records: Vec<EventRecord>,
    next_seq: u64,
}

impl EventLog for MemoryEventLog {
    fn append(&mut self, event: Event) -> EventRecord {
        let record = EventRecord {
            seq: self.next_seq,
            event,
        };
        self.next_seq += 1;
        self.records.push(record.clone());
        record
    }

    fn len(&self) -> usize {
        self.records.len()
    }

    fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn tail(&self, limit: usize) -> Vec<EventRecord> {
        let start = self.records.len().saturating_sub(limit);
        self.records[start..].to_vec()
    }

    fn since(&self, seq: u64) -> Vec<EventRecord> {
        self.records
            .iter()
            .filter(|r| r.seq > seq)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn make_event(text: &str) -> Event {
        Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: text.to_string(),
        }
    }

    #[test]
    fn starts_empty() {
        let log = MemoryEventLog::default();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn append_increments_seq() {
        let mut log = MemoryEventLog::default();

        let r0 = log.append(make_event("first"));
        let r1 = log.append(make_event("second"));
        let r2 = log.append(make_event("third"));

        assert_eq!(r0.seq, 0);
        assert_eq!(r1.seq, 1);
        assert_eq!(r2.seq, 2);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn tail_returns_last_n() {
        let mut log = MemoryEventLog::default();
        for i in 0..5 {
            log.append(make_event(&format!("event-{i}")));
        }

        let tail = log.tail(3);
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].seq, 2);
        assert_eq!(tail[1].seq, 3);
        assert_eq!(tail[2].seq, 4);
    }

    #[test]
    fn tail_more_than_len_returns_all() {
        let mut log = MemoryEventLog::default();
        log.append(make_event("only"));

        let tail = log.tail(100);
        assert_eq!(tail.len(), 1);
    }

    #[test]
    fn since_filters_by_seq() {
        let mut log = MemoryEventLog::default();
        for i in 0..5 {
            log.append(make_event(&format!("event-{i}")));
        }

        let since_2 = log.since(2);
        assert_eq!(since_2.len(), 2);
        assert_eq!(since_2[0].seq, 3);
        assert_eq!(since_2[1].seq, 4);
    }

    #[test]
    fn since_beyond_end_returns_empty() {
        let mut log = MemoryEventLog::default();
        log.append(make_event("a"));
        assert!(log.since(100).is_empty());
    }

    #[test]
    fn event_record_accessors() {
        let event = make_event("hello");
        let expected_id = event.id();
        let record = EventRecord { seq: 42, event };

        assert_eq!(record.id(), expected_id);
        assert_eq!(record.seq, 42);
    }
}
