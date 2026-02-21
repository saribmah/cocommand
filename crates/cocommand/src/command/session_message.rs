use crate::message::{ExtensionPart, FilePartSourceText, MessagePart, PartBase, TextPart};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionCommandInputPart {
    Text(SessionCommandTextPartInput),
    Extension(SessionCommandExtensionPartInput),
    File(SessionCommandFilePartInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SessionCommandTextPartInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SessionCommandExtensionPartInput {
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SessionCommandFilePartInput {
    pub path: String,
    pub name: String,
    #[serde(rename = "entryType", skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceText>,
}

pub(crate) fn map_input_parts(
    parts: Vec<SessionCommandInputPart>,
    session_id: &str,
    message_id: &str,
) -> Vec<MessagePart> {
    parts
        .into_iter()
        .map(|part| match part {
            SessionCommandInputPart::Text(part) => MessagePart::Text(TextPart {
                base: PartBase::new(session_id, message_id),
                text: part.text,
            }),
            SessionCommandInputPart::Extension(part) => MessagePart::Extension(ExtensionPart {
                base: PartBase::new(session_id, message_id),
                extension_id: part.extension_id,
                name: part.name,
                kind: part.kind,
                source: part.source,
            }),
            SessionCommandInputPart::File(part) => MessagePart::Text(TextPart {
                base: PartBase::new(session_id, message_id),
                text: map_file_input_to_text(part),
            }),
        })
        .collect()
}

fn map_file_input_to_text(part: SessionCommandFilePartInput) -> String {
    if !part.path.trim().is_empty() {
        return part.path;
    }

    if let Some(source) = part.source {
        if !source.value.trim().is_empty() {
            return source.value;
        }
    }

    if !part.name.trim().is_empty() {
        return format!("#{}", part.name);
    }

    "#file".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessagePart;

    #[test]
    fn map_input_parts_maps_text_extension_and_file_inputs() {
        let parts = map_input_parts(
            vec![
                SessionCommandInputPart::Extension(SessionCommandExtensionPartInput {
                    extension_id: "filesystem".to_string(),
                    name: "Filesystem".to_string(),
                    kind: Some("built-in".to_string()),
                    source: Some(FilePartSourceText {
                        value: "@filesystem".to_string(),
                        start: 0,
                        end: 11,
                    }),
                }),
                SessionCommandInputPart::Text(SessionCommandTextPartInput {
                    text: "hello".to_string(),
                }),
                SessionCommandInputPart::File(SessionCommandFilePartInput {
                    path: "/tmp/note.md".to_string(),
                    name: "note.md".to_string(),
                    entry_type: Some("file".to_string()),
                    source: Some(FilePartSourceText {
                        value: "#note.md".to_string(),
                        start: 0,
                        end: 8,
                    }),
                }),
            ],
            "session-1",
            "message-1",
        );

        assert!(matches!(
            parts.first(),
            Some(MessagePart::Extension(part))
                if part.extension_id == "filesystem"
                    && part.name == "Filesystem"
                    && part.kind.as_deref() == Some("built-in")
                    && part
                        .source
                        .as_ref()
                        .is_some_and(|source| source.value == "@filesystem")
        ));
        assert!(matches!(
            parts.get(1),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));
        assert!(matches!(
            parts.get(2),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "/tmp/note.md"
        ));
    }

    #[test]
    fn map_file_input_to_text_uses_fallback_order() {
        let from_path = map_file_input_to_text(SessionCommandFilePartInput {
            path: "/tmp/a.md".to_string(),
            name: "a.md".to_string(),
            entry_type: None,
            source: Some(FilePartSourceText {
                value: "#a.md".to_string(),
                start: 0,
                end: 5,
            }),
        });
        assert_eq!(from_path, "/tmp/a.md");

        let from_source = map_file_input_to_text(SessionCommandFilePartInput {
            path: "  ".to_string(),
            name: "a.md".to_string(),
            entry_type: None,
            source: Some(FilePartSourceText {
                value: "#a.md".to_string(),
                start: 0,
                end: 5,
            }),
        });
        assert_eq!(from_source, "#a.md");

        let from_name = map_file_input_to_text(SessionCommandFilePartInput {
            path: "  ".to_string(),
            name: "a.md".to_string(),
            entry_type: None,
            source: None,
        });
        assert_eq!(from_name, "#a.md");

        let fallback = map_file_input_to_text(SessionCommandFilePartInput {
            path: "  ".to_string(),
            name: "  ".to_string(),
            entry_type: None,
            source: None,
        });
        assert_eq!(fallback, "#file");
    }
}
