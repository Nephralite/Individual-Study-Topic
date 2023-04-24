use crate::initialization::QueueFamilies;
use ash::vk;

use crate::model::{InstanceData, Model};
use crate::rendering::Pipeline;
use crate::swapchain::Swapchain;

pub(crate) struct Pools {
    commandpool_graphics: vk::CommandPool,
    commandpool_transfer: vk::CommandPool,
}

impl Pools {
    pub(crate) fn init(
        logical_device: &ash::Device,
        queue_families: &QueueFamilies,
    ) -> Result<Pools, vk::Result> {
        let graphics_commandpool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index.unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let commandpool_graphics =
            unsafe { logical_device.create_command_pool(&graphics_commandpool_info, None) }?;
        let transfer_commandpool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index.unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let commandpool_transfer =
            unsafe { logical_device.create_command_pool(&transfer_commandpool_info, None) }?;

        Ok(Pools {
            commandpool_graphics,
            commandpool_transfer,
        })
    }
    pub(crate) fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_command_pool(self.commandpool_graphics, None);
            logical_device.destroy_command_pool(self.commandpool_transfer, None);
        }
    }
}

pub(crate) fn create_commandbuffers(
    logical_device: &ash::Device,
    pools: &Pools,
    amount: usize,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let commandbuf_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.commandpool_graphics)
        .command_buffer_count(amount as u32);
    unsafe { logical_device.allocate_command_buffers(&commandbuf_allocate_info) }
}

pub(crate) fn fill_commandbuffers(
    commandbuffers: &[vk::CommandBuffer],
    logical_device: &ash::Device,
    renderpass: &vk::RenderPass,
    swapchain: &Swapchain,
    pipeline: &Pipeline,
    models: &Vec<Model<[f32; 3], InstanceData>>,
) -> Result<(), vk::Result> {
    for (i, &command_buffer) in commandbuffers.iter().enumerate() {
        let commandbuffer_begininfo = vk::CommandBufferBeginInfo::default();
        unsafe {
            logical_device.begin_command_buffer(command_buffer, &commandbuffer_begininfo)?;
        }
        let clearvalues = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.08, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
            .render_pass(*renderpass)
            .framebuffer(swapchain.framebuffers[i])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent,
            })
            .clear_values(&clearvalues);
        unsafe {
            logical_device.cmd_begin_render_pass(
                command_buffer,
                &renderpass_begininfo,
                vk::SubpassContents::INLINE,
            );
            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            );
            for m in models {
                m.draw(logical_device, command_buffer);
            }
            logical_device.cmd_end_render_pass(command_buffer);
            logical_device.end_command_buffer(command_buffer)?;
        }
    }
    Ok(())
}
