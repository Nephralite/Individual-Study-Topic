use crate::buffer::Buffer;
use crate::commandbuffers::{create_commandbuffers, Pools};
use crate::debug::Debug;
use crate::initialization::{
    get_physical_device_and_properties, init_device_and_queues, init_instance, QueueFamilies,
    Queues,
};
use crate::model::{InstanceData, Model};
use crate::rendering::{init_renderpass, Pipeline};
use crate::surface::Surface;
use crate::swapchain::Swapchain;
use ash::{vk, Entry};
use nalgebra as na;

pub(crate) struct VkInterface {
    pub(crate) window: winit::window::Window,
    entry: Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<Debug>,
    surface: std::mem::ManuallyDrop<Surface>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_families: QueueFamilies,
    pub(crate) device: ash::Device,
    pub(crate) queues: Queues,
    pub(crate) swapchain: Swapchain,
    renderpass: vk::RenderPass,
    pipeline: Pipeline,
    pools: Pools,
    pub(crate) command_buffers: Vec<vk::CommandBuffer>,
    pub(crate) allocator: std::mem::ManuallyDrop<vk_mem::Allocator>,
    pub(crate) models: Vec<Model<[f32; 3], InstanceData>>,
    pub(crate) uniformbuffer: Buffer,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl VkInterface {
    pub(crate) fn init(
        window: winit::window::Window,
    ) -> Result<VkInterface, Box<dyn std::error::Error>> {
        let entry = unsafe { Entry::load()? };
        //TODO - requires validation layers to be installed on your machine
        let layer_names = vec![std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let instance = init_instance(&entry, &layer_names)?;
        let debug = Debug::init(&entry, &instance)?;
        let surface = Surface::init(&window, &entry, &instance)?;
        //grab my discrete GPU, which I know has all the features I need
        let (physical_device, physical_device_properties) =
            get_physical_device_and_properties(&instance).unwrap();
        let queue_families = QueueFamilies::init(&instance, physical_device)?;
        let (device, queues) =
            init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;
        let allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            std::rc::Rc::new(&instance),
            std::rc::Rc::new(&device),
            physical_device,
        );
        let allocator = vk_mem::Allocator::new(allocator_create_info)?;
        let mut swapchain = Swapchain::init(
            &instance,
            physical_device,
            &device,
            &surface,
            &queue_families,
            &allocator,
        )?;
        let renderpass = init_renderpass(&device, swapchain.surface_format.format)?;
        swapchain.create_framebuffers(&device, renderpass)?;
        let pipeline = Pipeline::init(&device, &swapchain, &renderpass)?;
        let pools = Pools::init(&device, &queue_families)?;
        let command_buffers =
            create_commandbuffers(&device, &pools, swapchain.amount_of_images as usize)?;

        let mut uniformbuffer = Buffer::new(
            &allocator,
            128,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        )?;
        let cameratransform: [[[f32; 4]; 4]; 2] = [
            na::Matrix4::identity().into(),
            na::Matrix4::identity().into(),
        ];
        unsafe { uniformbuffer.fill(&allocator, &cameratransform)? };
        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: swapchain.amount_of_images,
        }];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(swapchain.amount_of_images)
            .pool_sizes(&pool_sizes);
        let descriptor_pool =
            unsafe { device.create_descriptor_pool(&descriptor_pool_info, None) }?;

        let desc_layouts =
            vec![pipeline.descriptor_set_layouts[0]; swapchain.amount_of_images as usize];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desc_layouts);
        let descriptor_sets =
            unsafe { device.allocate_descriptor_sets(&descriptor_set_allocate_info) }?;

        for (_i, descset) in descriptor_sets.iter().enumerate() {
            let buffer_infos = [vk::DescriptorBufferInfo {
                buffer: uniformbuffer.buffer,
                offset: 0,
                range: 128,
            }];
            let desc_sets_write = [vk::WriteDescriptorSet::builder()
                .dst_set(*descset)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build()];
            unsafe { device.update_descriptor_sets(&desc_sets_write, &[]) };
        }

        Ok(VkInterface {
            window,
            entry,
            instance,
            debug: std::mem::ManuallyDrop::new(debug),
            surface: std::mem::ManuallyDrop::new(surface),
            physical_device,
            physical_device_properties,
            queue_families,
            device,
            queues,
            swapchain,
            renderpass,
            pipeline,
            pools,
            command_buffers,
            allocator: std::mem::ManuallyDrop::new(allocator),
            models: vec![],
            uniformbuffer,
            descriptor_pool,
            descriptor_sets,
        })
    }
    pub(crate) fn update_commandbuffer(&mut self, index: usize) -> Result<(), vk::Result> {
        let commandbuffer = self.command_buffers[index];
        let commandbuffer_begininfo = vk::CommandBufferBeginInfo::builder();
        unsafe {
            self.device
                .begin_command_buffer(commandbuffer, &commandbuffer_begininfo)?;
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
            .render_pass(self.renderpass)
            .framebuffer(self.swapchain.framebuffers[index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            })
            .clear_values(&clearvalues);
        unsafe {
            self.device.cmd_begin_render_pass(
                commandbuffer,
                &renderpass_begininfo,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                commandbuffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                commandbuffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[self.descriptor_sets[index]],
                &[],
            );
            for m in &self.models {
                m.draw(&self.device, commandbuffer);
            }
            self.device.cmd_end_render_pass(commandbuffer);
            self.device.end_command_buffer(commandbuffer)?;
        }
        Ok(())
    }
}

//TODO - a semaphore isn't dropped?, also allocs being horrible
impl Drop for VkInterface {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("something wrong while waiting");

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            /*
            self.allocator
                .destroy_buffer(self.uniformbuffer.buffer, self.uniformbuffer.allocation);
            for m in self.models {
                if let Some(vb) = &m.vertexbuffer {
                    self.allocator
                        .destroy_buffer(vb.buffer, vb.allocation)
                        .expect("problem with buffer destruction");
                }
                if let Some(ib) = &m.instancebuffer {
                    self.allocator
                        .destroy_buffer(ib.buffer, ib.allocation)
                        .expect("problem with buffer destruction");
                }
            }*/

            self.pools.cleanup(&self.device);
            self.pipeline.cleanup(&self.device);
            self.device.destroy_render_pass(self.renderpass, None);
            self.swapchain.cleanup(&self.device /*&self.allocator*/);
            std::mem::ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
            std::mem::ManuallyDrop::drop(&mut self.surface);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            self.instance.destroy_instance(None)
        };
    }
}
