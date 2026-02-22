use serde_json::{Map, Value};

use crate::error::CoreResult;
use crate::event::{CoreEvent, SessionPartUpdatedPayload};
use crate::message::message::MessageStorage;
use crate::message::{
    PartBase, ToolPart, ToolState, ToolStateCompleted, ToolStateError, ToolStateRunning,
    ToolStateTimeCompleted, ToolStateTimeRange, ToolStateTimeStart,
};
use crate::utils::time::now_secs;

use super::{value_to_string, SessionRuntimeActor, ToolCallRecord};

impl SessionRuntimeActor {
    pub(super) async fn write_tool_running_metadata(
        &self,
        record: &ToolCallRecord,
        job_id: &str,
    ) -> CoreResult<()> {
        let mut metadata = Map::new();
        metadata.insert("job_id".to_string(), Value::String(job_id.to_string()));
        metadata.insert("status".to_string(), Value::String("running".to_string()));

        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Running(ToolStateRunning {
                input: record.input.clone(),
                title: None,
                metadata: Some(metadata),
                time: ToolStateTimeStart {
                    start: record.started_at,
                },
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }

    pub(super) async fn write_tool_completed(
        &self,
        record: &ToolCallRecord,
        output: Value,
    ) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Completed(ToolStateCompleted {
                input: record.input.clone(),
                output: value_to_string(&output),
                title: record.tool_name.clone(),
                metadata: Map::new(),
                time: ToolStateTimeCompleted {
                    start: record.started_at,
                    end,
                    compacted: None,
                },
                attachments: None,
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }

    pub(super) async fn write_tool_error(
        &self,
        record: &ToolCallRecord,
        error: Value,
    ) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Error(ToolStateError {
                input: record.input.clone(),
                error: value_to_string(&error),
                metadata: None,
                time: ToolStateTimeRange {
                    start: record.started_at,
                    end,
                },
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }
}
