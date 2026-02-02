use crate::draw::UiTexId;

/// Reserved texture IDs for engine-level UI.
pub mod reserved {
    use super::UiTexId;

    /// Font atlas texture ID.
    pub const FONT_ATLAS: UiTexId = UiTexId(1);

    /// First user-allocated texture ID.
    pub const USER_BEGIN: u32 = 16;
}

/// Monotonic texture ID allocator owned by UI layer.
#[derive(Debug, Default)]
pub struct UiTexAllocator {
    next: u32,
}

impl UiTexAllocator {
    #[inline]
    pub fn new() -> Self {
        Self {
            next: reserved::USER_BEGIN,
        }
    }

    #[inline]
    pub fn alloc(&mut self) -> UiTexId {
        let id = UiTexId(self.next);
        self.next = self.next.saturating_add(1);
        id
    }
}