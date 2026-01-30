use std::time::Duration;

use moka::sync::Cache;

#[derive(Debug, Clone)]
pub struct ApplicationCache {
    cache: Cache<String, u64>,
}

impl ApplicationCache {
    pub fn new(max_applications: u32, ttl_seconds: u64) -> Self {
        let max_capacity = if max_applications == 0 { 1 } else { max_applications as u64 };
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    pub fn open_application(&self, app_id: &str, opened_at: u64) {
        self.cache.insert(app_id.to_string(), opened_at);
    }

    pub fn close_application(&self, app_id: &str) {
        self.cache.invalidate(app_id);
    }

    pub fn list_applications(&self) -> Vec<String> {
        self.cache
            .iter()
            .map(|(key, _)| (*key).clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_on_capacity() {
        let cache = ApplicationCache::new(2, 3600);
        cache.open_application("one", 1);
        cache.open_application("two", 2);
        cache.open_application("three", 3);

        let entries = cache.cache.entry_count();
        assert!(entries <= 2);
    }
}
