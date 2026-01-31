use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::application::ApplicationKind;
use crate::workspace::WorkspaceInstance;

pub fn build_search_applications_tool(workspace: Arc<WorkspaceInstance>) -> Tool {
    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let workspace = workspace.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            let query = input
                .get("query")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let limit = input
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(8) as usize;
            let registry = workspace.application_registry.read().await;
            let mut items: Vec<(serde_json::Value, i64)> = registry
                .list()
                .into_iter()
                .map(|app| {
                    let id = app.id().to_string();
                    let name = app.name().to_string();
                    let kind = map_kind(app.kind()).to_string();
                    let tags = app.tags();
                    let score = match_score(&query, &name, &id, &kind);
                    (
                        json!({
                            "id": id,
                            "name": name,
                            "kind": kind,
                            "tags": tags,
                        }),
                        score,
                    )
                })
                .filter(|(_, score)| query.is_empty() || *score >= 0)
                .collect();
            items.sort_by(|a, b| b.1.cmp(&a.1));
            let results: Vec<serde_json::Value> = items
                .into_iter()
                .take(limit)
                .map(|(value, _)| value)
                .collect();
            Ok(json!({ "results": results }))
        }))
    });

    Tool::function(json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" },
            "limit": { "type": "number", "minimum": 1, "maximum": 50 }
        },
        "required": ["query"]
    }))
    .with_description("Search available applications by name or id.")
    .with_execute(execute)
}

pub(crate) fn map_kind(kind: ApplicationKind) -> &'static str {
    match kind {
        ApplicationKind::System => "system",
        ApplicationKind::BuiltIn => "built-in",
        ApplicationKind::Custom => "custom",
    }
}

fn match_score(query: &str, name: &str, id: &str, kind: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let name_lower = name.to_lowercase();
    let id_lower = id.to_lowercase();
    let kind_lower = kind.to_lowercase();
    if name_lower.contains(query) || id_lower.contains(query) || kind_lower.contains(query) {
        return 100 + query.len() as i64;
    }
    let compact_query = query.replace(' ', "");
    let name_score = subsequence_score(&compact_query, &name_lower.replace(' ', ""));
    let id_score = subsequence_score(&compact_query, &id_lower.replace(' ', ""));
    let kind_score = subsequence_score(&compact_query, &kind_lower.replace(' ', ""));
    let best = name_score.max(id_score).max(kind_score);
    if best > 0 { best } else { -1 }
}

fn subsequence_score(query: &str, target: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let mut score = 0;
    let mut ti = 0;
    for ch in query.chars() {
        if let Some(found) = target[ti..].find(ch) {
            let index = ti + found;
            score += if index == ti { 2 } else { 1 };
            ti = index + 1;
        } else {
            return -1;
        }
    }
    score
}
