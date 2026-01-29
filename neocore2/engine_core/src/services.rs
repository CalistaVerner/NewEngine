use log::Log;

/// Хэндл текстуры, чтобы модули не знали конкретную графическую реализацию.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextureHandle(pub u64);

/// Интерфейс графики (без реализации).
pub trait GfxService: Send {
    fn create_texture_rgba8(&mut self, width: u32, height: u32, label: &str) -> anyhow::Result<TextureHandle>;

    fn update_texture_rgba8(&mut self, tex: TextureHandle, width: u32, height: u32, bytes: &[u8]) -> anyhow::Result<()>;

    fn update_texture_rgba8_region(
        &mut self,
        tex: TextureHandle,
        tex_width: u32,
        tex_height: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        bytes_rgba_tightly_packed: &[u8],
    ) -> anyhow::Result<()> {
        let _ = (x, y, w, h, tex_width, tex_height);
        self.update_texture_rgba8(tex, tex_width, tex_height, bytes_rgba_tightly_packed)
    }
}

/// Services = только “инфраструктура, не зависящая от Resources”.
/// GfxService достаём через Resources, чтобы не было двойных &mut.
pub trait Services {
    fn logger(&self) -> &dyn Log;
}