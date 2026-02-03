use crate::error::VkResult;

use ash::vk;

use super::state::VulkanRenderer;
use super::types::FRAMES_IN_FLIGHT;

use super::super::util::transition_image;

impl VulkanRenderer {
    pub fn draw_clear_color(&mut self, rgba: [f32; 4]) -> VkResult<()> {
        if self.target_width == 0 || self.target_height == 0 {
            return Ok(());
        }

        let frame = self.frames[self.frame_index];

        unsafe {
            self.device
                .wait_for_fences(&[frame.in_flight], true, u64::MAX)?;
        }

        let (image_index, _suboptimal) = match unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                frame.image_available,
                vk::Fence::null(),
            )
        } {
            Ok(v) => v,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => unsafe {
                self.recreate_swapchain()?;
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        let idx = image_index as usize;

        unsafe {
            let inflight = self.images_in_flight[idx];
            if inflight != vk::Fence::null() {
                self.device.wait_for_fences(&[inflight], true, u64::MAX)?;
            }
            self.images_in_flight[idx] = frame.in_flight;
            self.device.reset_fences(&[frame.in_flight])?;
        }

        let cmd = self.command_buffers[idx];
        let image = self.swapchain_images[idx];

        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            self.device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let old_layout = self.image_layouts[idx];

            transition_image(
                &self.device,
                cmd,
                image,
                old_layout,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );

            let clear = vk::ClearValue {
                color: vk::ClearColorValue { float32: rgba },
            };

            let rp_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[idx])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.extent,
                })
                .clear_values(std::slice::from_ref(&clear));

            self.device
                .cmd_begin_render_pass(cmd, &rp_begin, vk::SubpassContents::INLINE);

            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.extent.width as f32,
                height: self.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            };

            self.device
                .cmd_set_viewport(cmd, 0, std::slice::from_ref(&viewport));
            self.device
                .cmd_set_scissor(cmd, 0, std::slice::from_ref(&scissor));

            let t = self.start_time.elapsed().as_secs_f32();
            let aspect = self.extent.width as f32 / self.extent.height.max(1) as f32;
            let pc: [f32; 4] = [t, aspect, 0.0, 0.0];

            let pc_bytes: &[u8] =
                std::slice::from_raw_parts(pc.as_ptr() as *const u8, std::mem::size_of_val(&pc));

            self.device.cmd_push_constants(
                cmd,
                self.pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                pc_bytes,
            );

            self.device.cmd_draw(cmd, 3, 1, 0, 0);

            if self.text_pipeline != vk::Pipeline::null()
                && self.text_pipeline_layout != vk::PipelineLayout::null()
                && self.text_desc_set != vk::DescriptorSet::null()
                && self.text_vb != vk::Buffer::null()
            {
                let debug_text = std::mem::take(&mut self.debug_text);
                let res = self.draw_text_overlay(cmd, &debug_text);
                self.debug_text = debug_text;
                res?;
            }

            if let Some(list) = self.pending_ui.take() {
                let ui_ready = self.ui_pipeline != vk::Pipeline::null()
                    && self.ui_pipeline_layout != vk::PipelineLayout::null()
                    && self.ui_desc_set_layout != vk::DescriptorSetLayout::null()
                    && self.ui_sampler != vk::Sampler::null();

                if ui_ready {
                    self.ui_upload_and_draw(cmd, &list)?;
                }
            }

            self.device.cmd_end_render_pass(cmd);

            transition_image(
                &self.device,
                cmd,
                image,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );

            self.image_layouts[idx] = vk::ImageLayout::PRESENT_SRC_KHR;

            self.device.end_command_buffer(cmd)?;

            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let wait_sems = [frame.image_available];
            let signal_sems = [frame.render_finished];
            let cmd_bufs = [cmd];

            let submit_infos = [vk::SubmitInfo::default()
                .wait_semaphores(&wait_sems)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&cmd_bufs)
                .signal_semaphores(&signal_sems)];

            self.device
                .queue_submit(self.queue, &submit_infos, frame.in_flight)?;

            let swapchains = [self.swapchain];
            let indices = [image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_sems)
                .swapchains(&swapchains)
                .image_indices(&indices);

            match self
                .swapchain_loader
                .queue_present(self.queue, &present_info)
            {
                Ok(_) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                    self.recreate_swapchain()?;
                    self.frame_index = (self.frame_index + 1) % FRAMES_IN_FLIGHT;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            }
        }

        self.frame_index = (self.frame_index + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }
}
