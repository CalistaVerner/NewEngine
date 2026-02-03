use crate::error::VkResult;

use ash::vk;
use std::mem;
use std::ptr;

use super::super::device::*;
use super::super::util::*;
use super::super::VulkanRenderer;

use newengine_ui::draw::{UiDrawCmd, UiDrawList, UiTexId, UiTextureDelta};

use super::pipeline::{create_ui_pipeline, ui_pc_bytes};

#[derive(Clone, Copy)]
pub(crate) struct GpuUiTexture {
    pub(crate) image: vk::Image,
    pub(crate) mem: vk::DeviceMemory,
    pub(crate) view: vk::ImageView,
    pub(crate) desc_set: vk::DescriptorSet,
    pub(crate) size: [u32; 2],
}

impl VulkanRenderer {
    pub(crate) fn init_ui_overlay(&mut self) -> VkResult<()> {
        unsafe {
            self.create_ui_descriptor()?;
            let (pl, p) =
                create_ui_pipeline(&self.device, self.render_pass, self.ui_desc_set_layout)?;
            self.ui_pipeline_layout = pl;
            self.ui_pipeline = p;
        }
        Ok(())
    }

    pub(crate) unsafe fn destroy_ui_overlay(&mut self) {
        self.destroy_ui_resources();

        if self.ui_pipeline != vk::Pipeline::null() {
            self.device.destroy_pipeline(self.ui_pipeline, None);
        }
        if self.ui_pipeline_layout != vk::PipelineLayout::null() {
            self.device
                .destroy_pipeline_layout(self.ui_pipeline_layout, None);
        }

        if self.ui_desc_pool != vk::DescriptorPool::null() {
            self.device.destroy_descriptor_pool(self.ui_desc_pool, None);
        }
        if self.ui_desc_set_layout != vk::DescriptorSetLayout::null() {
            self.device
                .destroy_descriptor_set_layout(self.ui_desc_set_layout, None);
        }
        if self.ui_sampler != vk::Sampler::null() {
            self.device.destroy_sampler(self.ui_sampler, None);
        }

        if self.ui_vb != vk::Buffer::null() {
            self.device.destroy_buffer(self.ui_vb, None);
        }
        if self.ui_vb_mem != vk::DeviceMemory::null() {
            self.device.free_memory(self.ui_vb_mem, None);
        }
        if self.ui_ib != vk::Buffer::null() {
            self.device.destroy_buffer(self.ui_ib, None);
        }
        if self.ui_ib_mem != vk::DeviceMemory::null() {
            self.device.free_memory(self.ui_ib_mem, None);
        }
    }

    unsafe fn destroy_ui_resources(&mut self) {
        for (_id, tex) in self.ui_textures.drain() {
            if tex.view != vk::ImageView::null() {
                self.device.destroy_image_view(tex.view, None);
            }
            if tex.image != vk::Image::null() {
                self.device.destroy_image(tex.image, None);
            }
            if tex.mem != vk::DeviceMemory::null() {
                self.device.free_memory(tex.mem, None);
            }
        }
    }

    unsafe fn create_ui_descriptor(&mut self) -> VkResult<()> {
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);

        self.ui_sampler = self.device.create_sampler(&sampler_info, None)?;

        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        self.ui_desc_set_layout = self.device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding)),
            None,
        )?;

        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1024);

        self.ui_desc_pool = self.device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .max_sets(1024)
                .pool_sizes(std::slice::from_ref(&pool_size)),
            None,
        )?;

        Ok(())
    }

    pub(super) unsafe fn ui_apply_delta(&mut self, delta: &UiTextureDelta) -> VkResult<()> {
        for (id, tex) in &delta.set {
            self.ui_create_or_replace_texture(*id, tex.size, &tex.rgba8)?;
        }

        for patch in &delta.patches {
            self.ui_patch_texture(patch.id, patch.origin, patch.size, &patch.rgba8)?;
        }

        for id in &delta.free {
            self.ui_free_texture(*id);
        }

        Ok(())
    }

    unsafe fn ui_free_texture(&mut self, id: UiTexId) {
        if let Some(tex) = self.ui_textures.remove(&id.0) {
            self.device.destroy_image_view(tex.view, None);
            self.device.destroy_image(tex.image, None);
            self.device.free_memory(tex.mem, None);
        }
    }

    unsafe fn ui_create_or_replace_texture(
        &mut self,
        id: UiTexId,
        size: [u32; 2],
        rgba8: &[u8],
    ) -> VkResult<()> {
        self.ui_free_texture(id);

        let (w, h) = (size[0], size[1]);
        let expected = (w as usize) * (h as usize) * 4;
        if rgba8.len() != expected {
            return Err(vk::Result::ERROR_VALIDATION_FAILED_EXT.into());
        }

        let staging_size = rgba8.len() as vk::DeviceSize;
        let (staging_buf, staging_mem) = create_buffer(
            &self.instance,
            self.physical_device,
            &self.device,
            staging_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped =
            self.device
                .map_memory(staging_mem, 0, staging_size, vk::MemoryMapFlags::empty())?
                as *mut u8;
        ptr::copy_nonoverlapping(rgba8.as_ptr(), mapped, rgba8.len());
        self.device.unmap_memory(staging_mem);

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: w,
                height: h,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = self.device.create_image(&image_info, None)?;
        let req = self.device.get_image_memory_requirements(image);

        let mem_type = find_memory_type(
            &self.instance,
            self.physical_device,
            req.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(mem_type);

        let mem = self.device.allocate_memory(&alloc, None)?;
        self.device.bind_image_memory(image, mem, 0)?;

        immediate_submit(&self.device, self.upload_command_pool, self.queue, |cmd| {
            transition_image_layout(
                &self.device,
                cmd,
                image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: w,
                    height: h,
                    depth: 1,
                });

            self.device.cmd_copy_buffer_to_image(
                cmd,
                staging_buf,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&region),
            );

            transition_image_layout(
                &self.device,
                cmd,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            );
        })?;

        self.device.destroy_buffer(staging_buf, None);
        self.device.free_memory(staging_mem, None);

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        let view = self.device.create_image_view(&view_info, None)?;

        let layouts = [self.ui_desc_set_layout];
        let desc_set = self.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.ui_desc_pool)
                .set_layouts(&layouts),
        )?[0];

        let image_info = vk::DescriptorImageInfo::default()
            .sampler(self.ui_sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let write = vk::WriteDescriptorSet::default()
            .dst_set(desc_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));

        self.device
            .update_descriptor_sets(std::slice::from_ref(&write), &[]);

        self.ui_textures.insert(
            id.0,
            GpuUiTexture {
                image,
                mem,
                view,
                desc_set,
                size,
            },
        );

        Ok(())
    }

    unsafe fn ui_patch_texture(
        &mut self,
        id: UiTexId,
        origin: [u32; 2],
        size: [u32; 2],
        rgba8: &[u8],
    ) -> VkResult<()> {
        let Some(tex) = self.ui_textures.get(&id.0) else {
            return Ok(());
        };

        let (w, h) = (size[0], size[1]);
        let expected = (w as usize) * (h as usize) * 4;
        if rgba8.len() != expected {
            return Err(vk::Result::ERROR_VALIDATION_FAILED_EXT.into());
        }

        let staging_size = rgba8.len() as vk::DeviceSize;
        let (staging_buf, staging_mem) = create_buffer(
            &self.instance,
            self.physical_device,
            &self.device,
            staging_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped =
            self.device
                .map_memory(staging_mem, 0, staging_size, vk::MemoryMapFlags::empty())?
                as *mut u8;
        ptr::copy_nonoverlapping(rgba8.as_ptr(), mapped, rgba8.len());
        self.device.unmap_memory(staging_mem);

        immediate_submit(&self.device, self.upload_command_pool, self.queue, |cmd| {
            transition_image_layout(
                &self.device,
                cmd,
                tex.image,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .image_offset(vk::Offset3D {
                    x: origin[0] as i32,
                    y: origin[1] as i32,
                    z: 0,
                })
                .image_extent(vk::Extent3D {
                    width: w,
                    height: h,
                    depth: 1,
                });

            self.device.cmd_copy_buffer_to_image(
                cmd,
                staging_buf,
                tex.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&region),
            );

            transition_image_layout(
                &self.device,
                cmd,
                tex.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            );
        })?;

        self.device.destroy_buffer(staging_buf, None);
        self.device.free_memory(staging_mem, None);

        Ok(())
    }

    pub(super) unsafe fn ui_ensure_buffers(
        &mut self,
        vb_bytes: vk::DeviceSize,
        ib_bytes: vk::DeviceSize,
    ) -> VkResult<()> {
        if self.ui_vb == vk::Buffer::null() || vb_bytes > self.ui_vb_size {
            if self.ui_vb != vk::Buffer::null() {
                self.device.destroy_buffer(self.ui_vb, None);
            }
            if self.ui_vb_mem != vk::DeviceMemory::null() {
                self.device.free_memory(self.ui_vb_mem, None);
            }

            self.ui_vb_size = vb_bytes.max(64 * 1024);
            let (buf, mem) = create_buffer(
                &self.instance,
                self.physical_device,
                &self.device,
                self.ui_vb_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.ui_vb = buf;
            self.ui_vb_mem = mem;
        }

        if self.ui_ib == vk::Buffer::null() || ib_bytes > self.ui_ib_size {
            if self.ui_ib != vk::Buffer::null() {
                self.device.destroy_buffer(self.ui_ib, None);
            }
            if self.ui_ib_mem != vk::DeviceMemory::null() {
                self.device.free_memory(self.ui_ib_mem, None);
            }

            self.ui_ib_size = ib_bytes.max(64 * 1024);
            let (buf, mem) = create_buffer(
                &self.instance,
                self.physical_device,
                &self.device,
                self.ui_ib_size,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.ui_ib = buf;
            self.ui_ib_mem = mem;
        }

        Ok(())
    }

    pub(crate) unsafe fn ui_upload_and_draw(
        &mut self,
        cmd: vk::CommandBuffer,
        list: &UiDrawList,
    ) -> VkResult<()> {
        self.ui_apply_delta(&list.texture_delta)?;

        let vb_bytes = (mem::size_of::<newengine_ui::draw::UiVertex>() * list.mesh.vertices.len())
            as vk::DeviceSize;
        let ib_bytes = (mem::size_of::<u32>() * list.mesh.indices.len()) as vk::DeviceSize;

        self.ui_ensure_buffers(vb_bytes, ib_bytes)?;

        if !list.mesh.vertices.is_empty() {
            let mapped =
                self.device
                    .map_memory(self.ui_vb_mem, 0, vb_bytes, vk::MemoryMapFlags::empty())?
                    as *mut u8;
            ptr::copy_nonoverlapping(
                list.mesh.vertices.as_ptr() as *const u8,
                mapped,
                vb_bytes as usize,
            );
            self.device.unmap_memory(self.ui_vb_mem);
        }

        if !list.mesh.indices.is_empty() {
            let mapped =
                self.device
                    .map_memory(self.ui_ib_mem, 0, ib_bytes, vk::MemoryMapFlags::empty())?
                    as *mut u8;
            ptr::copy_nonoverlapping(
                list.mesh.indices.as_ptr() as *const u8,
                mapped,
                ib_bytes as usize,
            );
            self.device.unmap_memory(self.ui_ib_mem);
        }

        if list.mesh.indices.is_empty()
            || list.mesh.vertices.is_empty()
            || list.mesh.cmds.is_empty()
        {
            return Ok(());
        }

        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.ui_pipeline);

        let pc = ui_pc_bytes(list.screen_size_px);

        self.device.cmd_push_constants(
            cmd,
            self.ui_pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            &pc,
        );

        let vb = [self.ui_vb];
        let offsets = [0u64];
        self.device.cmd_bind_vertex_buffers(cmd, 0, &vb, &offsets);
        self.device
            .cmd_bind_index_buffer(cmd, self.ui_ib, 0, vk::IndexType::UINT32);

        for c in &list.mesh.cmds {
            self.ui_draw_cmd(cmd, c)?;
        }

        Ok(())
    }

    unsafe fn ui_draw_cmd(&mut self, cmd: vk::CommandBuffer, c: &UiDrawCmd) -> VkResult<()> {
        let Some(tex) = self.ui_textures.get(&c.texture.0) else {
            return Ok(());
        };

        let mut x0 = c.clip_rect.min_x.floor() as i32;
        let mut y0 = c.clip_rect.min_y.floor() as i32;
        let mut x1 = c.clip_rect.max_x.ceil() as i32;
        let mut y1 = c.clip_rect.max_y.ceil() as i32;

        x0 = x0.clamp(0, self.extent.width as i32);
        y0 = y0.clamp(0, self.extent.height as i32);
        x1 = x1.clamp(0, self.extent.width as i32);
        y1 = y1.clamp(0, self.extent.height as i32);

        if x1 <= x0 || y1 <= y0 {
            return Ok(());
        }

        let sc = vk::Rect2D {
            offset: vk::Offset2D { x: x0, y: y0 },
            extent: vk::Extent2D {
                width: (x1 - x0) as u32,
                height: (y1 - y0) as u32,
            },
        };

        self.device
            .cmd_set_scissor(cmd, 0, std::slice::from_ref(&sc));

        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.ui_pipeline_layout,
            0,
            std::slice::from_ref(&tex.desc_set),
            &[],
        );

        let first_index = c.index_range.start;
        let index_count = c.index_range.end.saturating_sub(c.index_range.start);

        if index_count == 0 {
            return Ok(());
        }

        self.device
            .cmd_draw_indexed(cmd, index_count, 1, first_index, 0, 0);
        Ok(())
    }
}
