use serde_json::{Map, Value};

use crate::bus::Bus;
use crate::command::runtime::protocol::ToolExecutionContext;
use crate::error::CoreResult;
use crate::event::{CoreEvent, SessionPartUpdatedPayload};
use crate::message::message::MessageStorage;
use crate::message::{
    MessagePart, PartBase, ToolPart, ToolState, ToolStateCompleted, ToolStateError,
    ToolStateTimeCompleted, ToolStateTimeRange,
};
use crate::storage::SharedStorage;
use crate::utils::time::now_secs;

#[derive(Clone)]
pub(super) struct ToolProcessor {
    storage: SharedStorage,
    bus: Bus,
}

impl ToolProcessor {
    pub(super) fn new(storage: SharedStorage, bus: Bus) -> Self {
        Self { storage, bus }
    }

    pub(super) async fn apply_completed(
        &self,
        context: &ToolExecutionContext,
        output: Value,
    ) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: context.part_id.clone(),
                session_id: context.session_id.clone(),
                message_id: context.message_id.clone(),
            },
            call_id: context.tool_call_id.clone(),
            tool: context.tool_name.clone(),
            state: ToolState::Completed(ToolStateCompleted {
                input: context.input.clone(),
                output: value_to_string(&output),
                title: context.tool_name.clone(),
                metadata: Map::new(),
                time: ToolStateTimeCompleted {
                    start: context.started_at,
                    end,
                    compacted: None,
                },
                attachments: None,
            }),
            metadata: None,
        };

        self.store_part(context, part).await
    }

    pub(super) async fn apply_error(
        &self,
        context: &ToolExecutionContext,
        error: Value,
    ) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: context.part_id.clone(),
                session_id: context.session_id.clone(),
                message_id: context.message_id.clone(),
            },
            call_id: context.tool_call_id.clone(),
            tool: context.tool_name.clone(),
            state: ToolState::Error(ToolStateError {
                input: context.input.clone(),
                error: value_to_string(&error),
                metadata: None,
                time: ToolStateTimeRange {
                    start: context.started_at,
                    end,
                },
            }),
            metadata: None,
        };

        self.store_part(context, part).await
    }

    async fn store_part(&self, context: &ToolExecutionContext, part: ToolPart) -> CoreResult<()> {
        MessageStorage::store_part(&self.storage, &MessagePart::Tool(part.clone())).await?;

        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: context.session_id.clone(),
                run_id: context.run_id.clone(),
                message_id: context.message_id.clone(),
                part_id: context.part_id.clone(),
                part: MessagePart::Tool(part),
            }));

        Ok(())
    }
}

pub(super) fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}
