use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct CefViewId(pub u64);

/// Minimal CEF API exposed to engine modules.
pub trait CefApi: Send + Sync {
    /// Returns true if runtime is initialized.
    fn is_ready(&self) -> bool;

    /// Creates (or returns) a primary view.
    fn ensure_primary_view(&self) -> CefViewId;

    /// Loads local HTML into the primary view.
    fn load_local_html(&self, html: &str);

    /// Loads URL into the primary view.
    fn load_url(&self, url: &str);

    /// Evaluates JS in the primary view.
    fn eval_js(&self, js: &str);

    /// Requests focus for the view.
    fn focus(&self, focused: bool);

    /// Resize notification.
    fn resize(&self, width: u32, height: u32);
}

pub type CefApiRef = Arc<dyn CefApi + Send + Sync>;