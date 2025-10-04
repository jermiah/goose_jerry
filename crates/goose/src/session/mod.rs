pub mod extension_data;
pub mod legacy;
pub mod session_manager;
pub mod tool_classifier;

pub use session_manager::{ensure_session_dir, Session, SessionInsights, SessionManager, ToolStats};
pub use tool_classifier::{classify_tool, ToolOperation};
