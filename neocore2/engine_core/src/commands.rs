/// Простой CommandQueue без аллокаций в кадре (если заранее reserve).
/// По мере роста можно заменить на ring-buffer или slab + indices.
pub struct CommandQueue<T> {
    buf: Vec<T>,
}

impl<T> CommandQueue<T> {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { buf: Vec::with_capacity(cap) }
    }

    /// Рекомендуется вызвать один раз при старте, чтобы не аллоцировать в кадре.
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    #[inline]
    pub fn push(&mut self, cmd: T) {
        self.buf.push(cmd);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Забрать накопленные команды, применить, очистить.
    /// Паттерн: subsystem.prepare(commands.drain()).
    #[inline]
    pub fn drain(&mut self) -> std::vec::Drain<'_, T> {
        self.buf.drain(..)
    }

    /// Очистка между кадрами, если нужно.
    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl<T> Default for CommandQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}