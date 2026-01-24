//! Append-only in-memory event store.

use super::event::Event;

/// An append-only, ordered store of events.
///
/// Events are stored in insertion order. This is the v0 in-memory
/// implementation with no persistence.
///
/// Deprecated: Use [`crate::storage::EventLog`] trait via [`crate::storage::MemoryStorage`] instead.
#[deprecated(note = "Use storage::EventLog trait via MemoryStorage instead")]
pub struct EventStore {
    events: Vec<Event>,
}

impl EventStore {
    /// Create a new empty event store.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append an event to the store.
    pub fn append(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Returns a slice of all events in insertion order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Returns the number of events in the store.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the store contains no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns a slice of events starting from the given index.
    ///
    /// If `index` is beyond the end, returns an empty slice.
    pub fn events_since(&self, index: usize) -> &[Event] {
        if index >= self.events.len() {
            &[]
        } else {
            &self.events[index..]
        }
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn make_user_message(text: &str) -> Event {
        Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: text.to_string(),
        }
    }

    fn make_error(code: &str, message: &str) -> Event {
        Event::ErrorRaised {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    #[test]
    fn new_store_is_empty() {
        let store = EventStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.events().is_empty());
    }

    #[test]
    fn append_preserves_order() {
        let mut store = EventStore::new();
        let e1 = make_user_message("first");
        let e2 = make_user_message("second");
        let e3 = make_user_message("third");

        let id1 = e1.id();
        let id2 = e2.id();
        let id3 = e3.id();

        store.append(e1);
        store.append(e2);
        store.append(e3);

        assert_eq!(store.len(), 3);
        assert!(!store.is_empty());
        assert_eq!(store.events()[0].id(), id1);
        assert_eq!(store.events()[1].id(), id2);
        assert_eq!(store.events()[2].id(), id3);
    }

    #[test]
    fn events_since_returns_tail() {
        let mut store = EventStore::new();
        store.append(make_user_message("a"));
        store.append(make_user_message("b"));
        store.append(make_error("E1", "err"));

        let since_1 = store.events_since(1);
        assert_eq!(since_1.len(), 2);

        let since_3 = store.events_since(3);
        assert!(since_3.is_empty());

        let since_100 = store.events_since(100);
        assert!(since_100.is_empty());
    }

    #[test]
    fn events_since_zero_returns_all() {
        let mut store = EventStore::new();
        store.append(make_user_message("x"));
        store.append(make_user_message("y"));

        assert_eq!(store.events_since(0).len(), 2);
    }

    #[test]
    fn default_is_empty() {
        let store = EventStore::default();
        assert!(store.is_empty());
    }
}
