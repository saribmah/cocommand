//! Search functionality for filesystem indexes.
//!
//! This module provides:
//! - The main FileSystemIndexManager API
//! - Search engine for querying indexed data

mod engine;
mod manager;

// Re-export main types
pub use engine::search_index_data;
pub use manager::FileSystemIndexManager;
