use ash::vk;

use super::state::VulkanRenderer;

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.destroy_ui_overlay();
            self.destroy_text_overlay();

            if self.upload_command_pool != vk::CommandPool::null() {
                self.device
                    .destroy_command_pool(self.upload_command_pool, None);
                self.upload_command_pool = vk::CommandPool::null();
            }

            for f in &self.frames {
                if f.in_flight != vk::Fence::null() {
                    self.device.destroy_fence(f.in_flight, None);
                }
                if f.render_finished != vk::Semaphore::null() {
                    self.device.destroy_semaphore(f.render_finished, None);
                }
                if f.image_available != vk::Semaphore::null() {
                    self.device.destroy_semaphore(f.image_available, None);
                }
            }

            if self.command_pool != vk::CommandPool::null() {
                if !self.command_buffers.is_empty() {
                    self.device
                        .free_command_buffers(self.command_pool, &self.command_buffers);
                }
                self.device.destroy_command_pool(self.command_pool, None);
                self.command_pool = vk::CommandPool::null();
            }

            for &fb in &self.framebuffers {
                if fb != vk::Framebuffer::null() {
                    self.device.destroy_framebuffer(fb, None);
                }
            }
            self.framebuffers.clear();

            if self.pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.pipeline, None);
                self.pipeline = vk::Pipeline::null();
            }
            if self.pipeline_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.pipeline_layout, None);
                self.pipeline_layout = vk::PipelineLayout::null();
            }
            if self.render_pass != vk::RenderPass::null() {
                self.device.destroy_render_pass(self.render_pass, None);
                self.render_pass = vk::RenderPass::null();
            }

            for &iv in &self.swapchain_image_views {
                if iv != vk::ImageView::null() {
                    self.device.destroy_image_view(iv, None);
                }
            }
            self.swapchain_image_views.clear();

            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
                self.swapchain = vk::SwapchainKHR::null();
            }

            if self.surface != vk::SurfaceKHR::null() {
                self.surface_loader.destroy_surface(self.surface, None);
                self.surface = vk::SurfaceKHR::null();
            }

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
