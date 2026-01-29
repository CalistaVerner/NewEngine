use std::sync::Arc;

/// Minimal CEF API exposed to engine modules.
///
/// Must stay stable.
/// Higher-level UI abstractions should live elsewhere.
pub trait CefApi: Send + Sync {
    fn load_local_html(&self, html: &str);
    fn eval_js(&self, js: &str);
}

/// Shared reference to CEF API implementation.
pub type CefApiRef = Arc<dyn CefApi + Send + Sync>;

/// Resource key for storing CefApiRef inside Resources.
///
/// Stored value type:
///   Arc<CefApiRef>  (i.e. Arc<Arc<dyn CefApi>>)
///
/// This is intentional and matches Resources<T = Arc<_>> design.
pub struct CefApiKey;