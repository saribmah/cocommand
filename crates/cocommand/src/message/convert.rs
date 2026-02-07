use llm_kit_core::output::Output;
use llm_kit_core::stream_text::StreamTextResult;

use crate::error::{CoreError, CoreResult};
use crate::message::parts::{
    FilePart, MessagePart, ReasoningPart, SourcePart, TextPart, ToolCallPart, ToolErrorPart,
    ToolResultPart,
};
use llm_kit_provider::language_model::content::source::LanguageModelSource;

pub fn outputs_to_parts(outputs: &[Output]) -> Vec<MessagePart> {
    outputs
        .iter()
        .filter_map(|output| match output {
            Output::Text(text) => Some(MessagePart::Text(TextPart {
                text: text.text.clone(),
            })),
            Output::Reasoning(reasoning) => Some(MessagePart::Reasoning(ReasoningPart {
                text: reasoning.text.clone(),
            })),
            Output::ToolCall(call) => Some(MessagePart::ToolCall(ToolCallPart {
                call_id: call.tool_call_id.clone(),
                tool_name: call.tool_name.clone(),
                input: call.input.clone(),
            })),
            Output::ToolResult(result) => Some(MessagePart::ToolResult(ToolResultPart {
                call_id: result.tool_call_id.clone(),
                tool_name: result.tool_name.clone(),
                output: result.output.clone(),
                is_error: false,
            })),
            Output::ToolError(error) => Some(MessagePart::ToolError(ToolErrorPart {
                call_id: error.tool_call_id.clone(),
                tool_name: error.tool_name.clone(),
                error: error.error.clone(),
            })),
            Output::Source(source) => Some(MessagePart::Source(map_source(source))),
            Output::File(file) => Some(MessagePart::File(FilePart {
                base64: file.base64().to_string(),
                media_type: file.media_type.clone(),
                name: file.name.clone(),
            })),
        })
        .collect()
}

pub async fn stream_result_to_parts(result: &StreamTextResult) -> CoreResult<Vec<MessagePart>> {
    let content = result
        .content()
        .await
        .map_err(|error| CoreError::Internal(error.to_string()))?;
    Ok(outputs_to_parts(&content))
}

fn map_source(source: &llm_kit_core::output::SourceOutput) -> SourcePart {
    match &source.source {
        LanguageModelSource::Url { id, url, title, .. } => SourcePart {
            id: id.clone(),
            source_type: "url".to_string(),
            url: Some(url.clone()),
            title: title.clone(),
            media_type: None,
            filename: None,
        },
        LanguageModelSource::Document {
            id,
            media_type,
            title,
            filename,
            ..
        } => SourcePart {
            id: id.clone(),
            source_type: "document".to_string(),
            url: None,
            title: Some(title.clone()),
            media_type: Some(media_type.clone()),
            filename: filename.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_kit_provider_utils::tool::ToolError;
    use serde_json::json;

    #[test]
    fn outputs_to_parts_maps_tool_error_to_tool_error_part() {
        let outputs = vec![Output::ToolError(ToolError::new(
            "call_1",
            "test_tool",
            json!({"input": true}),
            json!({"message": "failed"}),
        ))];

        let parts = outputs_to_parts(&outputs);
        assert!(matches!(
            parts.first(),
            Some(MessagePart::ToolError(part))
            if part.call_id == "call_1"
                && part.tool_name == "test_tool"
                && part.error == json!({"message": "failed"})
        ));
    }
}
