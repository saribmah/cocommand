use crate::message::{Message, MessagePart};
use crate::session::SessionContext;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", content = "payload")]
pub enum CoreEvent {
    SessionMessageStarted(SessionMessageStartedPayload),
    SessionPartUpdated(SessionPartUpdatedPayload),
    SessionRunCompleted(SessionRunCompletedPayload),
    SessionRunCancelled(SessionRunCancelledPayload),
    BackgroundJobStarted(BackgroundJobStartedPayload),
    BackgroundJobCompleted(BackgroundJobCompletedPayload),
    BackgroundJobFailed(BackgroundJobFailedPayload),
    SessionContextUpdated(SessionContextPayload),
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionMessageStartedPayload {
    pub session_id: String,
    pub run_id: String,
    pub user_message: Option<Message>,
    pub assistant_message: Message,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionPartUpdatedPayload {
    pub session_id: String,
    pub run_id: String,
    pub message_id: String,
    pub part_id: String,
    pub part: MessagePart,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionContextPayload {
    pub session_id: String,
    pub run_id: Option<String>,
    pub context: SessionContext,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionRunCompletedPayload {
    pub session_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionRunCancelledPayload {
    pub session_id: String,
    pub run_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BackgroundJobStartedPayload {
    pub session_id: String,
    pub run_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BackgroundJobCompletedPayload {
    pub session_id: String,
    pub run_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BackgroundJobFailedPayload {
    pub session_id: String,
    pub run_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub job_id: String,
    pub error: String,
}
