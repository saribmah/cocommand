//! Query context types for matching against nodes.

use std::borrow::Cow;

use crate::slab::{NodeFileType, SlabNode};

use super::path::{extension_of_name, normalize_path_for_compare, split_path_segments};

/// Query context for SlabNode-based matching.
///
/// This computes all needed data from the compact node storage
/// for query matching and filtering.
pub struct NodeQueryContext {
    name: Cow<'static, str>,
    path: Cow<'static, str>,
    comparable_path: String,
    path_segments: Vec<String>,
    extension: Option<String>,
    file_type: NodeFileType,
    size: Option<u64>,
    modified_at: Option<u64>,
    created_at: Option<u64>,
}

impl NodeQueryContext {
    /// Creates a new context from a SlabNode and its computed path.
    pub fn new(node: &SlabNode, path: String, case_sensitive: bool) -> Self {
        let name_str = node.name();
        let name: Cow<'static, str> = if case_sensitive {
            Cow::Borrowed(name_str)
        } else {
            Cow::Owned(name_str.to_ascii_lowercase())
        };
        let path_cow: Cow<'static, str> = if case_sensitive {
            Cow::Owned(path.clone())
        } else {
            Cow::Owned(path.to_ascii_lowercase())
        };
        let comparable_path = normalize_path_for_compare(path_cow.as_ref());
        let path_segments = split_path_segments(comparable_path.as_str());

        Self {
            name,
            path: path_cow,
            comparable_path,
            path_segments,
            extension: extension_of_name(name_str),
            file_type: node.file_type(),
            size: node.size(),
            modified_at: node.modified_at(),
            created_at: node.created_at(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }

    pub fn comparable_path(&self) -> &str {
        self.comparable_path.as_str()
    }

    pub fn path_segments(&self) -> &[String] {
        self.path_segments.as_slice()
    }

    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// Returns the modification time as Unix timestamp (seconds).
    pub fn modified_at(&self) -> Option<u64> {
        self.modified_at
    }

    /// Returns the creation time as Unix timestamp (seconds).
    pub fn created_at(&self) -> Option<u64> {
        self.created_at
    }

    /// Returns the file type for filter matching.
    #[allow(dead_code)]
    pub fn file_type(&self) -> NodeFileType {
        self.file_type
    }

    /// Returns true if this is a file.
    pub fn is_file(&self) -> bool {
        self.file_type == NodeFileType::File
    }

    /// Returns true if this is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type == NodeFileType::Dir
    }
}
