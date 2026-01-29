pub mod error;
pub mod server;
pub mod workspace;
pub mod sessions;

pub use crate::error::{CoreError, CoreResult};
pub use crate::sessions::{
    close_application, get_session_context, open_application, record_user_message, SessionContext,
    SessionMessage,
};
pub use crate::workspace::WorkspaceInstance;
