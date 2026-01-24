/// Result of parsing a raw command string.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    /// The original unmodified input.
    pub raw_text: String,
    /// Input after trimming, collapsing whitespace, and removing tags.
    pub normalized_text: String,
    /// Extracted `@app` tags in order of appearance, lowercased.
    pub tags: Vec<String>,
}
