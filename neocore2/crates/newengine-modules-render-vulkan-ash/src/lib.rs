mod error;
mod vulkan;

use crate::error::VkRenderError;
use crate::vulkan::VulkanRenderer;

use newengine_core::{EngineError, EngineResult, Module, ModuleCtx};
use newengine_platform_winit::{WinitWindowHandles, WinitWindowInitSize};

pub struct VulkanAshRenderModule {
    renderer: Option<VulkanRenderer>,
}

impl Default for VulkanAshRenderModule {
    fn default() -> Self {
        Self { renderer: None }
    }
}

impl<E: Send + 'static> Module<E> for VulkanAshRenderModule {
    fn id(&self) -> &'static str {
        "render.vulkan.ash"
    }

    fn init(&mut self, ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        let handles = ctx
            .resources()
            .get::<WinitWindowHandles>()
            .ok_or_else(|| EngineError::Other(VkRenderError::MissingWindow.to_string()))?;

        let size = ctx
            .resources()
            .get::<WinitWindowInitSize>()
            .ok_or_else(|| EngineError::Other("Missing WinitWindowInitSize".to_string()))?;

        let renderer = unsafe { VulkanRenderer::new(handles.display, handles.window, size.width, size.height) }
            .map_err(|e| EngineError::Other(e.to_string()))?;

        self.renderer = Some(renderer);
        Ok(())
    }

    fn render(&mut self, ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        let Some(r) = self.renderer.as_mut() else {
            return Ok(());
        };

        // Always track latest window size from platform resource.
        if let Some(sz) = ctx.resources().get::<WinitWindowInitSize>() {
            r.set_target_size(sz.width, sz.height);
        }

        r.draw_clear().map_err(|e| EngineError::Other(e.to_string()))?;
        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        self.renderer = None;
        Ok(())
    }
}