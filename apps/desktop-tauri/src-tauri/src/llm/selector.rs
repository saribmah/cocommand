use crate::applications::{ApplicationDefinition, ToolDefinition};

#[derive(Clone)]
pub struct AppSelection {
    pub app_id: String,
    pub confidence: f32,
}

#[derive(Clone)]
pub struct ToolSelection {
    pub tool_id: String,
    pub confidence: f32,
}

pub fn select_application(query: &str, apps: &[ApplicationDefinition]) -> Option<AppSelection> {
    let normalized = normalize(query);
    let mut best: Option<(String, f32)> = None;

    for app in apps {
        let score = score_match(&app.name, &app.description, &normalized, Some(&app.id));
        if score <= 0.0 {
            continue;
        }
        if best.as_ref().map_or(true, |(_, best_score)| score > *best_score) {
            best = Some((app.id.clone(), score));
        }
    }

    best.map(|(app_id, confidence)| AppSelection { app_id, confidence })
}

pub fn select_tool(query: &str, tools: &[ToolDefinition]) -> Option<ToolSelection> {
    let normalized = normalize(query);
    let mut best: Option<(String, f32)> = None;

    for tool in tools {
        let score = score_match(&tool.name, &tool.description, &normalized, Some(&tool.id));
        if score <= 0.0 {
            continue;
        }
        if best.as_ref().map_or(true, |(_, best_score)| score > *best_score) {
            best = Some((tool.id.clone(), score));
        }
    }

    best.map(|(tool_id, confidence)| ToolSelection { tool_id, confidence })
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn score_match(name: &str, description: &str, query: &str, id: Option<&str>) -> f32 {
    if query.is_empty() {
        return 0.0;
    }
    let name_norm = normalize(name);
    let desc_norm = normalize(description);
    let id_norm = id.map(normalize);

    if name_norm == query {
        return 1.0;
    }
    if name_norm.contains(query) || query.contains(&name_norm) {
        return 0.8;
    }
    if let Some(id_norm) = id_norm.as_ref() {
        if query.contains(id_norm) {
            return 0.75;
        }
        let id_suffix = id_norm.split('.').last().unwrap_or(id_norm);
        if query.contains(id_suffix) {
            return 0.7;
        }
    }
    if !desc_norm.is_empty() && (desc_norm.contains(query) || query.contains(&desc_norm)) {
        return 0.5;
    }
    0.0
}
