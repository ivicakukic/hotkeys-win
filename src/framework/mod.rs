mod app_registry;
mod router;
mod traits;

// Re-export public interface
pub use app_registry::set_app_handler;
pub use router::wnd_proc_router;
pub use traits::{AppHandler, Window};