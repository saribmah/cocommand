use crate::message::MessagePart;
use crate::session::SessionContext;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", content = "payload")]
pub enum CoreEvent {
    SessionPartUpdated(SessionPartUpdatedPayload),
    SessionContextUpdated(SessionContextPayload),
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionPartUpdatedPayload {
    pub request_id: String,
    pub session_id: String,
    pub message_id: String,
    pub part_id: String,
    pub part: MessagePart,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionContextPayload {
    pub request_id: String,
    pub context: SessionContext,
}
