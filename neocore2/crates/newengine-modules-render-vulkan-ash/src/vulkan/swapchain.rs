use crate::error::VkResult;

use ash::vk;
use ash::Device;

use super::pipeline::*;
use super::text::*;
use super::VulkanRenderer;

/// Creates a swapchain. If `old_swapchain` is not null, Vulkan may reuse resources internally.
pub(super) fn create_swapchain(
    swapchain_loader: &ash::khr::swapchain::Device,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    width: u32,
    height: u32,
    queue_family_index: u32,
    old_swapchain: vk::SwapchainKHR,
) -> VkResult<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D)> {
    let caps = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    }?;

    let formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }?;

    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    }?;

    let surface_format = formats
        .iter()
        .cloned()
        .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
        .unwrap_or(formats[0]);

    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let extent = if caps.current_extent.width != u32::MAX {
        caps.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    };

    let image_count = (caps.min_image_count + 1).min(if caps.max_image_count == 0 {
        u32::MAX
    } else {
        caps.max_image_count
    });

    let family_indices = [queue_family_index];

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&family_indices)
        .pre_transform(caps.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain);

    let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };
    let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

    Ok((swapchain, images, surface_format.format, extent))
}

pub(super) fn create_image_views(
    device: &Device,
    images: &[vk::Image],
    format: vk::Format,
) -> VkResult<Vec<vk::ImageView>> {
    let mut views = Vec::with_capacity(images.len());
    for &image in images {
        let iv = unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
                None,
            )?
        };
        views.push(iv);
    }
    Ok(views)
}

impl VulkanRenderer {
    /// Recreates swapchain and all swapchain-dependent resources.
    ///
    /// Safety: must be called only when no command buffers are executing that reference old resources.
    pub(super) unsafe fn recreate_swapchain(&mut self) -> VkResult<()> {
        if self.target_width == 0 || self.target_height == 0 {
            return Ok(());
        }

        // Hard sync: simplest correctness. Later можно заменить на per-frame fences + device idle only when needed.
        let _ = self.device.device_wait_idle();

        for &fb in &self.framebuffers {
            self.device.destroy_framebuffer(fb, None);
        }
        self.framebuffers.clear();

        for &iv in &self.swapchain_image_views {
            self.device.destroy_image_view(iv, None);
        }
        self.swapchain_image_views.clear();

        // Keep old swapchain alive until new one is created.
        let old_swapchain = self.swapchain;

        let (new_swapchain, new_images, new_format, new_extent) = create_swapchain(
            &self.swapchain_loader,
            &self.surface_loader,
            self.surface,
            self.physical_device,
            self.target_width,
            self.target_height,
            self.queue_family_index,
            old_swapchain,
        )?;

        // Now it's safe to destroy old swapchain.
        if old_swapchain != vk::SwapchainKHR::null() {
            self.swapchain_loader.destroy_swapchain(old_swapchain, None);
        }

        let new_image_views = create_image_views(&self.device, &new_images, new_format)?;
        let new_image_count = new_images.len();
        let format_changed = new_format != self.swapchain_format;

        if format_changed {
            if self.pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.pipeline, None);
                self.pipeline = vk::Pipeline::null();
            }
            if self.pipeline_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.pipeline_layout, None);
                self.pipeline_layout = vk::PipelineLayout::null();
            }

            if self.text_pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.text_pipeline, None);
                self.text_pipeline = vk::Pipeline::null();
            }
            if self.text_pipeline_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.text_pipeline_layout, None);
                self.text_pipeline_layout = vk::PipelineLayout::null();
            }

            if self.ui_pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.ui_pipeline, None);
                self.ui_pipeline = vk::Pipeline::null();
            }
            if self.ui_pipeline_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.ui_pipeline_layout, None);
                self.ui_pipeline_layout = vk::PipelineLayout::null();
            }

            if self.render_pass != vk::RenderPass::null() {
                self.device.destroy_render_pass(self.render_pass, None);
                self.render_pass = vk::RenderPass::null();
            }

            self.swapchain_format = new_format;
            self.render_pass = create_render_pass(&self.device, self.swapchain_format)?;

            let (pl, p) = create_pipeline(&self.device, self.render_pass)?;
            self.pipeline_layout = pl;
            self.pipeline = p;

            // Text pipeline depends on render pass.
            if self.text_desc_set_layout != vk::DescriptorSetLayout::null() {
                let (tpl, tp) = create_text_pipeline(
                    &self.device,
                    self.render_pass,
                    self.text_desc_set_layout,
                )?;
                self.text_pipeline_layout = tpl;
                self.text_pipeline = tp;
            }

            // UI pipeline depends on render pass too.
            if self.ui_desc_set_layout != vk::DescriptorSetLayout::null() {
                let (upl, up) = super::ui::create_ui_pipeline(
                    &self.device,
                    self.render_pass,
                    self.ui_desc_set_layout,
                )?;
                self.ui_pipeline_layout = upl;
                self.ui_pipeline = up;
            }
        } else {
            self.swapchain_format = new_format;
        }

        let new_framebuffers =
            create_framebuffers(&self.device, self.render_pass, &new_image_views, new_extent)?;

        if self.command_pool != vk::CommandPool::null() && !self.command_buffers.is_empty() {
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
        }

        self.command_buffers = self.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(new_image_count as u32),
        )?;

        self.swapchain = new_swapchain;
        self.swapchain_images = new_images;
        self.extent = new_extent;
        self.swapchain_image_views = new_image_views;
        self.framebuffers = new_framebuffers;

        self.image_layouts = vec![vk::ImageLayout::UNDEFINED; new_image_count];
        self.images_in_flight = vec![vk::Fence::null(); new_image_count];

        Ok(())
    }
}
