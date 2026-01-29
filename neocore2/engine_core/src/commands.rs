#[derive(Debug, Clone)]
pub enum EngineEvent {
    ShutdownRequested,
    WindowResized { w: u32, h: u32 },
    FocusChanged { focused: bool },
}

/// Запросы (API между модулями).
/// Важный принцип: Engine НЕ знает смысл этих запросов.
/// Это просто enum сообщений, а маршрутизация — через Bus.
#[derive(Debug, Clone)]
pub enum Request {
    // --- Chromium/CEF API ---
    CefCreateView { view: String, width: u32, height: u32 },
    CefRenderHtml { view: String, html: String },
    CefNavigate { view: String, url: String },

    // Пример “универсальных” запросов (можно расширять):
    Ping { message: String },
}

#[derive(Debug, Clone)]
pub enum Response {
    Ok,
    Text(String),
    Err(String),
}
