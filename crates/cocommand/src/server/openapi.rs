use utoipa::OpenApi;

use crate::command::session_message::{
    SessionCommandExtensionPartInput, SessionCommandFilePartInput, SessionCommandInputPart,
    SessionCommandTextPartInput,
};
use crate::event::{CoreEvent, SessionContextPayload, SessionPartUpdatedPayload};
use crate::message::parts::{
    ExtensionPart, FilePart, FilePartFileSource, FilePartSource, FilePartSourceText,
    FilePartSymbolSource, MessagePart, PartBase, ReasoningPart, TextPart, ToolPart, ToolState,
    ToolStateCompleted, ToolStateError, ToolStatePending, ToolStateRunning, ToolStateTimeCompleted,
    ToolStateTimeRange, ToolStateTimeStart,
};
use crate::server::browser::BrowserStatus;
use crate::server::error::{ApiErrorBody, ApiErrorResponse};
use crate::server::extension::{
    ExtensionInfo, ExtensionToolInfo, ExtensionViewInfo, ExtensionViewPopoutInfo,
    OpenExtensionRequest,
};
use crate::server::oauth::{PollResponse, StartFlowRequest, StartFlowResponse};
use crate::server::session::{
    ApiSessionContext, RecordMessageRequest, RecordMessageResponse,
};
use crate::server::system::{
    ApplicationInfo, ApplicationsResponse, OpenApplicationRequest, OpenApplicationResponse,
};
use crate::session::SessionContext;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Cocommand API",
        version = "0.1.0",
        description = "AI-native command bar for macOS"
    ),
    paths(
        crate::server::session::session_command,
        crate::server::session::session_context,
        crate::server::extension::list_extensions,
        crate::server::extension::open_extension,
        crate::server::invoke::invoke_tool,
        crate::server::system::list_applications,
        crate::server::system::open_application,
        crate::server::browser::status,
        crate::server::events::stream_events,
        crate::server::oauth::start_flow,
        crate::server::oauth::callback,
        crate::server::oauth::poll_flow,
        crate::server::oauth::set_tokens,
        crate::server::oauth::get_tokens,
        crate::server::oauth::delete_tokens,
    ),
    components(schemas(
        // Error
        ApiErrorResponse,
        ApiErrorBody,
        // Session
        RecordMessageRequest,
        RecordMessageResponse,
        ApiSessionContext,
        SessionContext,
        // Command input parts
        SessionCommandInputPart,
        SessionCommandTextPartInput,
        SessionCommandExtensionPartInput,
        SessionCommandFilePartInput,
        // Message parts
        MessagePart,
        PartBase,
        TextPart,
        ReasoningPart,
        ToolPart,
        ExtensionPart,
        FilePart,
        // Tool state
        ToolState,
        ToolStatePending,
        ToolStateRunning,
        ToolStateCompleted,
        ToolStateError,
        ToolStateTimeStart,
        ToolStateTimeRange,
        ToolStateTimeCompleted,
        // File sources
        FilePartSourceText,
        FilePartSource,
        FilePartFileSource,
        FilePartSymbolSource,
        // Extensions
        ExtensionInfo,
        ExtensionToolInfo,
        ExtensionViewInfo,
        ExtensionViewPopoutInfo,
        OpenExtensionRequest,
        // System / Applications
        ApplicationInfo,
        ApplicationsResponse,
        OpenApplicationRequest,
        OpenApplicationResponse,
        // Browser
        BrowserStatus,
        // OAuth
        StartFlowRequest,
        StartFlowResponse,
        PollResponse,
        // Events
        CoreEvent,
        SessionPartUpdatedPayload,
        SessionContextPayload,
    )),
    tags(
        (name = "sessions", description = "Session and command management"),
        (name = "extensions", description = "Extension registry and invocation"),
        (name = "system", description = "System applications"),
        (name = "browser", description = "Browser bridge"),
        (name = "events", description = "Server-sent events"),
        (name = "oauth", description = "OAuth flow management"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_spec_matches_code() {
        let from_code = ApiDoc::openapi()
            .to_pretty_json()
            .expect("serialize spec");

        let spec_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/api-client/openapi.json");

        let on_disk = std::fs::read_to_string(&spec_path).unwrap_or_else(|_| {
            panic!(
                "Could not read checked-in spec at {}. Run `cargo run --bin generate_openapi` first.",
                spec_path.display()
            )
        });

        assert_eq!(
            from_code.trim(),
            on_disk.trim(),
            "The checked-in openapi.json is stale. Run `cargo run --bin generate_openapi` to update it."
        );
    }
}
