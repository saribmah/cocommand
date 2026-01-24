/// Routing metadata for an application.
///
/// Apps register this metadata with the router so commands can be
/// matched to candidate apps via keyword/verb/object/example overlap.
#[derive(Debug, Clone)]
pub struct RoutingMetadata {
    pub app_id: String,
    /// Keywords that trigger this app (e.g., ["copy", "paste", "clipboard"]).
    pub keywords: Vec<String>,
    /// Example commands this app can handle (e.g., ["copy this text", "paste from clipboard"]).
    pub examples: Vec<String>,
    /// Verbs this app responds to (e.g., ["copy", "paste", "cut"]).
    pub verbs: Vec<String>,
    /// Objects/nouns this app operates on (e.g., ["clipboard", "text", "selection"]).
    pub objects: Vec<String>,
}
