//! Type filter targets and extension category definitions.

/// Target for type-based filtering.
#[derive(Debug, Clone, Copy)]
pub enum TypeFilterTarget {
    File,
    Directory,
    Extensions(&'static [&'static str]),
}

// ---------------------------------------------------------------------------
// Extension category constants
// ---------------------------------------------------------------------------

pub const PICTURE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tif", "tiff", "webp", "ico", "svg", "heic", "heif", "raw",
    "arw", "cr2", "orf", "raf", "psd", "ai",
];

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv", "mpg", "mpeg", "3gp", "3g2", "ts",
    "mts", "m2ts",
];

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "oga", "opus", "wma", "m4a", "alac", "aiff",
];

pub const DOCUMENT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rst", "doc", "docx", "rtf", "odt", "pdf", "pages", "rtfd",
];

pub const PRESENTATION_EXTENSIONS: &[&str] = &["ppt", "pptx", "key", "odp"];

pub const SPREADSHEET_EXTENSIONS: &[&str] = &["xls", "xlsx", "csv", "numbers", "ods"];

pub const PDF_EXTENSIONS: &[&str] = &["pdf"];

pub const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "cab", "iso", "dmg",
];

pub const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "c", "cc", "cpp", "cxx", "h", "hpp", "hh", "java", "cs", "py",
    "go", "rb", "swift", "kt", "kts", "php", "html", "css", "scss", "sass", "less", "json", "yaml",
    "yml", "toml", "ini", "cfg", "sh", "zsh", "fish", "ps1", "psm1", "sql", "lua", "pl", "pm", "r",
    "m", "mm", "dart", "scala", "ex", "exs",
];

pub const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe", "msi", "bat", "cmd", "com", "ps1", "psm1", "app", "apk", "ipa", "jar", "bin", "run",
    "pkg",
];

// ---------------------------------------------------------------------------
// Type filter target lookup
// ---------------------------------------------------------------------------

/// Looks up a type filter target by name.
pub fn lookup_type_filter_target(value: &str) -> Option<TypeFilterTarget> {
    match value {
        "file" | "files" => Some(TypeFilterTarget::File),
        "folder" | "folders" | "dir" | "directory" => Some(TypeFilterTarget::Directory),
        "picture" | "pictures" | "image" | "images" | "photo" | "photos" => {
            Some(TypeFilterTarget::Extensions(PICTURE_EXTENSIONS))
        }
        "video" | "videos" | "movie" | "movies" => {
            Some(TypeFilterTarget::Extensions(VIDEO_EXTENSIONS))
        }
        "audio" | "audios" | "music" | "song" | "songs" => {
            Some(TypeFilterTarget::Extensions(AUDIO_EXTENSIONS))
        }
        "doc" | "docs" | "document" | "documents" | "text" | "office" => {
            Some(TypeFilterTarget::Extensions(DOCUMENT_EXTENSIONS))
        }
        "presentation" | "presentations" | "ppt" | "slides" => {
            Some(TypeFilterTarget::Extensions(PRESENTATION_EXTENSIONS))
        }
        "spreadsheet" | "spreadsheets" | "xls" | "excel" | "sheet" | "sheets" => {
            Some(TypeFilterTarget::Extensions(SPREADSHEET_EXTENSIONS))
        }
        "pdf" => Some(TypeFilterTarget::Extensions(PDF_EXTENSIONS)),
        "archive" | "archives" | "compressed" | "zip" => {
            Some(TypeFilterTarget::Extensions(ARCHIVE_EXTENSIONS))
        }
        "code" | "source" | "dev" => Some(TypeFilterTarget::Extensions(CODE_EXTENSIONS)),
        "exe" | "exec" | "executable" | "executables" | "program" | "programs" | "app" | "apps" => {
            Some(TypeFilterTarget::Extensions(EXECUTABLE_EXTENSIONS))
        }
        _ => None,
    }
}
