pub mod error;
pub mod server;
pub mod workspace;
pub mod sessions;

pub use crate::error::{CoreError, CoreResult};
pub use crate::sessions::{
    close_window, get_session_context, open_window, record_user_message, SessionContext,
    SessionMessage,
};
pub use crate::workspace::WorkspaceInstance;
