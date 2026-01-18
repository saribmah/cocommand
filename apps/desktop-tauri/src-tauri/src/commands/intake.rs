use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct CommandRequest {
    pub text: String,
    pub source: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct CommandInput {
    pub id: String,
    pub text: String,
    pub source: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

pub fn normalize(request: CommandRequest) -> CommandInput {
    let trimmed = request.text.trim().to_string();
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    let id = format!("cmd_{}_{}", slug_id(&trimmed), OffsetDateTime::now_utc().unix_timestamp_nanos());

    CommandInput {
        id,
        text: trimmed,
        source: request.source.unwrap_or_else(|| "ui".to_string()),
        created_at: timestamp,
    }
}

fn slug_id(value: &str) -> String {
    let normalized = value.trim().to_lowercase();
    let mut out = String::new();
    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            out.push('_');
        }
    }
    if out.is_empty() {
        "command".to_string()
    } else {
        out
    }
}
