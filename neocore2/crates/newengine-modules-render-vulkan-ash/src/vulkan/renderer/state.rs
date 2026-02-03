use ash::vk;
use ash::{Device, Instance};
use newengine_ui::draw::UiDrawList;
use std::time::Instant;

pub struct VulkanRenderer {
    pub(crate) instance: Instance,

    pub(crate) render_pass: vk::RenderPass,
    pub(super) debug_text: String,
    pub(crate) framebuffers: Vec<vk::Framebuffer>,

    pub(crate) pipeline_layout: vk::PipelineLayout,
    pub(crate) pipeline: vk::Pipeline,

    pub(crate) surface_loader: ash::khr::surface::Instance,
    pub(crate) surface: vk::SurfaceKHR,

    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) device: Device,

    pub(crate) queue_family_index: u32,
    pub(crate) queue: vk::Queue,

    pub(crate) swapchain_loader: ash::khr::swapchain::Device,
    pub(crate) swapchain: vk::SwapchainKHR,
    pub(crate) swapchain_images: Vec<vk::Image>,
    pub(crate) swapchain_image_views: Vec<vk::ImageView>,
    pub(crate) swapchain_format: vk::Format,
    pub(crate) extent: vk::Extent2D,

    pub(crate) upload_command_pool: vk::CommandPool,
    pub(crate) image_layouts: Vec<vk::ImageLayout>,

    pub(crate) command_pool: vk::CommandPool,
    pub(crate) command_buffers: Vec<vk::CommandBuffer>,

    pub(super) frames: [super::types::FrameSync; super::types::FRAMES_IN_FLIGHT],
    pub(super) frame_index: usize,
    pub(crate) images_in_flight: Vec<vk::Fence>,

    pub(crate) target_width: u32,
    pub(crate) target_height: u32,

    pub(super) start_time: Instant,

    pub(crate) text_pipeline_layout: vk::PipelineLayout,
    pub(crate) text_pipeline: vk::Pipeline,

    pub(crate) text_desc_set_layout: vk::DescriptorSetLayout,
    pub(crate) text_desc_pool: vk::DescriptorPool,
    pub(crate) text_desc_set: vk::DescriptorSet,

    pub(crate) font_image: vk::Image,
    pub(crate) font_image_mem: vk::DeviceMemory,
    pub(crate) font_image_view: vk::ImageView,
    pub(crate) font_sampler: vk::Sampler,

    pub(crate) text_vb: vk::Buffer,
    pub(crate) text_vb_mem: vk::DeviceMemory,
    pub(crate) text_vb_size: vk::DeviceSize,

    pub(super) pending_ui: Option<UiDrawList>,

    pub(crate) ui_pipeline_layout: vk::PipelineLayout,
    pub(crate) ui_pipeline: vk::Pipeline,

    pub(crate) ui_desc_set_layout: vk::DescriptorSetLayout,
    pub(crate) ui_desc_pool: vk::DescriptorPool,
    pub(crate) ui_sampler: vk::Sampler,

    pub(crate) ui_textures: std::collections::HashMap<u32, super::super::ui::GpuUiTexture>,

    pub(crate) ui_vb: vk::Buffer,
    pub(crate) ui_vb_mem: vk::DeviceMemory,
    pub(crate) ui_vb_size: vk::DeviceSize,

    pub(crate) ui_ib: vk::Buffer,
    pub(crate) ui_ib_mem: vk::DeviceMemory,
    pub(crate) ui_ib_size: vk::DeviceSize,

    pub(crate) ui_staging_buf: vk::Buffer,
    pub(crate) ui_staging_mem: vk::DeviceMemory,
    pub(crate) ui_staging_size: vk::DeviceSize,
}
