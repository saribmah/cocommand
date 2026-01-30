pub mod file;

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::error::CoreResult;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn write(&self, keys: &[&str], data: &Value) -> CoreResult<()>;
    async fn read(&self, keys: &[&str]) -> CoreResult<Option<Value>>;
    async fn list(&self, keys: &[&str]) -> CoreResult<Vec<String>>;
}

pub type SharedStorage = Arc<dyn Storage>;
