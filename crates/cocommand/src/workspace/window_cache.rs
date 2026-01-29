use std::time::Duration;

use moka::sync::Cache;

#[derive(Debug, Clone)]
pub struct WindowCache {
    cache: Cache<String, u64>,
}

impl WindowCache {
    pub fn new(max_windows: u32, ttl_seconds: u64) -> Self {
        let max_capacity = if max_windows == 0 { 1 } else { max_windows as u64 };
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    pub fn open_window(&self, window_id: &str, opened_at: u64) {
        self.cache.insert(window_id.to_string(), opened_at);
    }

    pub fn close_window(&self, window_id: &str) {
        self.cache.invalidate(window_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_on_capacity() {
        let cache = WindowCache::new(2, 3600);
        cache.open_window("one", 1);
        cache.open_window("two", 2);
        cache.open_window("three", 3);

        let entries = cache.cache.entry_count();
        assert!(entries <= 2);
    }
}
