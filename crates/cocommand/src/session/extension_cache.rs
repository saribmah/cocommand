use std::time::Duration;

use moka::sync::Cache;

#[derive(Debug, Clone)]
pub struct ExtensionCache {
    cache: Cache<String, u64>,
}

impl ExtensionCache {
    pub fn new(max_extensions: u32, ttl_seconds: u64) -> Self {
        let max_capacity = if max_extensions == 0 { 1 } else { max_extensions as u64 };
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    pub fn add(&self, app_id: &str, opened_at: u64) {
        self.cache.insert(app_id.to_string(), opened_at);
    }

    pub fn close_extension(&self, app_id: &str) {
        self.cache.invalidate(app_id);
    }

    pub fn list_extensions(&self) -> Vec<String> {
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
        let cache = ExtensionCache::new(2, 3600);
        cache.add("one", 1);
        cache.add("two", 2);
        cache.add("three", 3);

        let entries = cache.cache.entry_count();
        assert!(entries <= 2);
    }
}
