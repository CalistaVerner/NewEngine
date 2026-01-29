// module/services.rs

/// Engine-wide immutable services.
///
/// This is intentionally small and stable.
/// Extend via Resources if you need typed APIs.
pub trait Services: Send + Sync {
    fn logger(&self) -> &dyn log::Log;
}

/// Default engine services implementation.
pub struct EngineServices;

impl EngineServices {
    #[inline(always)]
    pub fn new() -> Self {
        Self
    }
}

impl Services for EngineServices {
    #[inline(always)]
    fn logger(&self) -> &dyn log::Log {
        log::logger()
    }
}