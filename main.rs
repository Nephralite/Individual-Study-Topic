use ash::vk;
use ash::Entry;
use std::ptr::null;
use vk_mem::Alloc;

/*#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    //color: [f32; 4],
}*/

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}

fn init_instance(
    entry: &Entry,
    layer_names: &Vec<std::ffi::CString>,
) -> Result<ash::Instance, vk::Result> {
    let app_name = std::ffi::CString::new("Jades Vulkan App").unwrap();
    let engine_name = std::ffi::CString::new("Jades Engine").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 0, 1))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::make_api_version(0, 1, 0, 0));
    let layer_name_pointers: Vec<*const i8> = layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let extension_name_pointers: Vec<*const i8> = vec![
        ash::extensions::ext::DebugUtils::name().as_ptr(),
        ash::extensions::khr::Surface::name().as_ptr(),
        ash::extensions::khr::XlibSurface::name().as_ptr(),
    ];
    let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback));
    //now actually make an instance
    let create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);
    unsafe { Ok(entry.create_instance(&create_info, None)?) }
}

struct Debug {
    utils: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Debug {
    fn init(entry: &Entry, instance: &ash::Instance) -> Result<Debug, vk::Result> {
        let utils = ash::extensions::ext::DebugUtils::new(entry, instance);
        let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));
        let messenger = unsafe { utils.create_debug_utils_messenger(&debug_create_info, None)? };
        Ok(Debug { utils, messenger })
    }
}

impl Drop for Debug {
    fn drop(&mut self) {
        unsafe {
            self.utils
                .destroy_debug_utils_messenger(self.messenger, None)
        };
    }
}

//TODO - This is OS specific to xorg linux, OS compatibility changes would need to be here
struct Surface {
    xlib_surface_loader: ash::extensions::khr::XlibSurface,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl Surface {
    fn init(
        window: &winit::window::Window,
        entry: &Entry,
        instance: &ash::Instance,
    ) -> Result<Surface, vk::Result> {
        use winit::platform::x11::WindowExtX11;
        let x11_display = window.xlib_display().unwrap();
        let x11_window = window.xlib_window().unwrap();
        let x11_create_info = vk::XlibSurfaceCreateInfoKHR::builder()
            .window(x11_window)
            .dpy(x11_display as *mut vk::Display);
        let xlib_surface_loader = ash::extensions::khr::XlibSurface::new(entry, instance);
        let surface = unsafe { xlib_surface_loader.create_xlib_surface(&x11_create_info, None) }?;
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
        Ok(Surface {
            xlib_surface_loader,
            surface,
            surface_loader,
        })
    }
    fn get_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }
    /*fn _get_present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
        }
    }*/
    fn get_formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

//TODO - This would not work on any integrated GPUs, also doesn't check for graphics support
fn get_physical_device_and_properties(
    instance: &ash::Instance,
) -> Result<(vk::PhysicalDevice, vk::PhysicalDeviceProperties), vk::Result> {
    let mut chosen = None;
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    for p in phys_devs {
        let properties = unsafe { instance.get_physical_device_properties(p) };
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            chosen = Some((p, properties));
        }
    }
    Ok(chosen.unwrap())
}

struct QueueFamilies {
    graphics_q_index: Option<u32>,
    transfer_q_index: Option<u32>,
}

impl QueueFamilies {
    fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        //surfaces: &Surface,
    ) -> Result<QueueFamilies, vk::Result> {
        let queuefamily_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, qfam) in queuefamily_properties.iter().enumerate() {
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                found_graphics_q_index = Some(index as u32);
            }
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                if found_transfer_q_index.is_none()
                    || !qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                {
                    found_transfer_q_index = Some(index as u32);
                }
            }
        }
        Ok(QueueFamilies {
            graphics_q_index: found_graphics_q_index,
            transfer_q_index: found_transfer_q_index,
        })
    }
}

struct Queues {
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue,
}

fn init_device_and_queues(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_families: &QueueFamilies,
    layer_names: &Vec<std::ffi::CString>,
) -> Result<(ash::Device, Queues), vk::Result> {
    let layer_name_pointers: Vec<*const i8> = layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let priorities = [1.0f32];
    let queue_info = [
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
    ];
    let device_extension_name_pointers: Vec<*const i8> =
        vec![ash::extensions::khr::Swapchain::name().as_ptr()];
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_info)
        .enabled_extension_names(&device_extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let graphics_queue =
        unsafe { logical_device.get_device_queue(queue_families.graphics_q_index.unwrap(), 0) };
    let transfer_queue =
        unsafe { logical_device.get_device_queue(queue_families.transfer_q_index.unwrap(), 0) };
    Ok((
        logical_device,
        Queues {
            graphics_queue,
            transfer_queue,
        },
    ))
}

struct Swapchain {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    imageviews: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    image_available: Vec<vk::Semaphore>,
    rendering_finished: Vec<vk::Semaphore>,
    may_begin_drawing: Vec<vk::Fence>,
    amount_of_images: u32,
    current_image: usize,
}

impl Swapchain {
    fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        logical_device: &ash::Device,
        surfaces: &Surface,
        queue_families: &QueueFamilies,
        //queues: &Queues,
    ) -> Result<Swapchain, vk::Result> {
        let surface_capabilities = surfaces.get_capabilities(physical_device)?;
        let extent = surface_capabilities.current_extent;
        //let surface_present_modes = surfaces.get_present_modes(physical_device)?;
        let surface_formats = *surfaces.get_formats(physical_device)?.last().unwrap(); //first was giving warnings
        let queuefamilies = [queue_families.graphics_q_index.unwrap()];
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surfaces.surface)
            .min_image_count(desired_image_count)
            .image_format(surface_formats.format)
            .image_color_space(surface_formats.color_space)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queuefamilies)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let mut swapchain_imageviews = Vec::with_capacity(swapchain_images.len());
        for image in &swapchain_images {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let imageview_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .subresource_range(*subresource_range);
            let imageview =
                unsafe { logical_device.create_image_view(&imageview_create_info, None) }?;
            swapchain_imageviews.push(imageview);
        }
        let mut image_available = vec![];
        let mut rendering_finished = vec![];
        let semaphoreinfo = vk::SemaphoreCreateInfo::builder();
        for _ in 0..desired_image_count {
            let semaphore_available =
                unsafe { logical_device.create_semaphore(&semaphoreinfo, None) }?;
            let semaphore_finished =
                unsafe { logical_device.create_semaphore(&semaphoreinfo, None) }?;
            image_available.push(semaphore_available);
            rendering_finished.push(semaphore_finished);
        }
        let mut image_available = vec![];
        let mut rendering_finished = vec![];
        let mut may_begin_drawing = vec![];
        let semaphoreinfo = vk::SemaphoreCreateInfo::builder();
        let fenceinfo = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        for _ in 0..desired_image_count {
            let semaphore_available =
                unsafe { logical_device.create_semaphore(&semaphoreinfo, None) }?;
            let semaphore_finished =
                unsafe { logical_device.create_semaphore(&semaphoreinfo, None) }?;
            image_available.push(semaphore_available);
            rendering_finished.push(semaphore_finished);
            let fence = unsafe { logical_device.create_fence(&fenceinfo, None) }?;
            may_begin_drawing.push(fence);
        }
        Ok(Swapchain {
            swapchain_loader,
            swapchain,
            images: swapchain_images,
            imageviews: swapchain_imageviews,
            framebuffers: vec![],
            surface_format: surface_formats,
            extent,
            amount_of_images: desired_image_count,
            image_available,
            rendering_finished,
            current_image: 0,
            may_begin_drawing,
        })
    }
    fn create_framebuffers(
        &mut self,
        logical_device: &ash::Device,
        renderpass: vk::RenderPass,
    ) -> Result<(), vk::Result> {
        for iv in &self.imageviews {
            let iview = [*iv];
            let framebuffer_info = vk::FramebufferCreateInfo::builder()
                .render_pass(renderpass)
                .attachments(&iview)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            let fb = unsafe { logical_device.create_framebuffer(&framebuffer_info, None) }?;
            self.framebuffers.push(fb);
        }
        Ok(())
    }
    unsafe fn cleanup(&mut self, logical_device: &ash::Device) {
        for fence in &self.may_begin_drawing {
            logical_device.destroy_fence(*fence, None);
        }
        for semaphore in &self.image_available {
            logical_device.destroy_semaphore(*semaphore, None);
        }
        for semaphore in &self.rendering_finished {
            logical_device.destroy_semaphore(*semaphore, None);
        }
        for fb in &self.framebuffers {
            logical_device.destroy_framebuffer(*fb, None);
        }
        for iv in &self.imageviews {
            logical_device.destroy_image_view(*iv, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None)
    }
}

fn init_renderpass(
    logical_device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    surfaces: &Surface,
) -> Result<vk::RenderPass, vk::Result> {
    let attachments = [vk::AttachmentDescription::builder()
        .format(
            surfaces
                .get_formats(physical_device)?
                .last()
                .unwrap()
                .format,
        )
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .samples(vk::SampleCountFlags::TYPE_1)
        .build()];
    let color_attachment_references = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_references)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build()];
    let subpass_dependencies = [vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_subpass(0)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
        .build()];
    let renderpass_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_dependencies);
    let renderpass = unsafe { logical_device.create_render_pass(&renderpass_info, None)? };
    Ok(renderpass)
}

struct Pipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Pipeline {
    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
            logical_device.destroy_pipeline_layout(self.layout, None);
        }
    }

    fn init(
        logical_device: &ash::Device,
        swapchain: &Swapchain,
        renderpass: &vk::RenderPass,
    ) -> Result<Pipeline, vk::Result> {
        let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
            vk_shader_macros::include_glsl!("./shaders/shader.vert", kind: vert),
        );
        let vertexshader_module =
            unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
        let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("./shaders/shader.frag"));
        let fragmentshader_module =
            unsafe { logical_device.create_shader_module(&fragmentshader_createinfo, None)? };
        let mainfunctionname = std::ffi::CString::new("main").unwrap();
        let vertexshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertexshader_module)
            .name(&mainfunctionname);
        let fragmentshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragmentshader_module)
            .name(&mainfunctionname);
        let shader_stages = vec![vertexshader_stage.build(), fragmentshader_stage.build()];
        let vertex_attrib_descs = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                offset: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 1,
                offset: 0,
                format: vk::Format::R32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 2,
                offset: 4,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
        ];
        let vertex_binding_descs = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: 16,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: 20,
                input_rate: vk::VertexInputRate::VERTEX,
            },

        ];
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attrib_descs)
            .vertex_binding_descriptions(&vertex_binding_descs);
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewports = [vk::Viewport {
            x: 0.,
            y: 0.,
            width: swapchain.extent.width as f32,
            height: swapchain.extent.height as f32,
            min_depth: 0.,
            max_depth: 1.,
        }];
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        }];

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL);
        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let colourblend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .build()];
        let colourblend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&colourblend_attachments);
        let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipelinelayout =
            unsafe { logical_device.create_pipeline_layout(&pipelinelayout_info, None) }?;
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .color_blend_state(&colourblend_info)
            .layout(pipelinelayout)
            .render_pass(*renderpass)
            .subpass(0);
        let graphicspipeline = unsafe {
            logical_device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_info.build()],
                    None,
                )
                .expect("A problem with the pipeline creation")
        }[0];
        unsafe {
            logical_device.destroy_shader_module(fragmentshader_module, None);
            logical_device.destroy_shader_module(vertexshader_module, None);
        }
        Ok(Pipeline {
            pipeline: graphicspipeline,
            layout: pipelinelayout,
        })
    }
}

struct Pools {
    commandpool_graphics: vk::CommandPool,
    commandpool_transfer: vk::CommandPool,
}

impl Pools {
    fn init(
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
    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_command_pool(self.commandpool_graphics, None);
            logical_device.destroy_command_pool(self.commandpool_transfer, None);
        }
    }
}

fn create_commandbuffers(
    logical_device: &ash::Device,
    pools: &Pools,
    amount: usize,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let commandbuf_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.commandpool_graphics)
        .command_buffer_count(amount as u32);
    unsafe { logical_device.allocate_command_buffers(&commandbuf_allocate_info) }
}

fn fill_commandbuffers(
    commandbuffers: &[vk::CommandBuffer],
    logical_device: &ash::Device,
    renderpass: &vk::RenderPass,
    swapchain: &Swapchain,
    pipeline: &Pipeline,
    vb1: &vk::Buffer,
    vb2: &vk::Buffer
) -> Result<(), vk::Result> {
    for (i, &command_buffer) in commandbuffers.iter().enumerate() {
        let commandbuffer_begininfo = vk::CommandBufferBeginInfo::default();
        unsafe {
            logical_device.begin_command_buffer(command_buffer, &commandbuffer_begininfo)?;
        }
        let clearvalues = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.08, 1.0],
            },
        }];
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
            logical_device.cmd_bind_vertex_buffers(command_buffer, 0, &[*vb1], &[0]);
            logical_device.cmd_bind_vertex_buffers(command_buffer, 1, &[*vb2], &[0]);
            logical_device.cmd_draw(command_buffer, 3, 1, 0, 0);
            logical_device.cmd_end_render_pass(command_buffer);
            logical_device.end_command_buffer(command_buffer)?;
        }
    }
    Ok(())
}

struct Buffer {
    buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
    //allocation_info: vk_mem::AllocationInfo,
}

impl Buffer {
    fn new(
        allocator: &vk_mem::Allocator,
        size_in_bytes: u64,
        usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Result<Buffer, vk::Result> {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            ..Default::default()
        };
        let (buffer, allocation) = unsafe { allocator.create_buffer(
            &ash::vk::BufferCreateInfo::builder()
                .size(size_in_bytes)
                .usage(usage)
                .build(),
            &allocation_create_info
        )?};
        Ok(Buffer {
            buffer,
            allocation,
            //allocation_info,
        })
    }
    unsafe fn fill<T: Sized>(
        &mut self,
        allocator: &vk_mem::Allocator,
        data: &[T],
    ) -> Result<(), vk::Result> {
        let data_ptr = allocator.map_memory(&mut self.allocation)? as *mut T;
        data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        allocator.unmap_memory(&mut self.allocation);
        Ok(())
    }
}


struct VkInterface {
    window: winit::window::Window,
    entry: Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<Debug>,
    surface: std::mem::ManuallyDrop<Surface>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_families: QueueFamilies,
    device: ash::Device,
    queues: Queues,
    swapchain: Swapchain,
    renderpass: vk::RenderPass,
    pipeline: Pipeline,
    pools: Pools,
    command_buffers: Vec<vk::CommandBuffer>,
    allocator: std::mem::ManuallyDrop<vk_mem::Allocator>,
    buffers: Vec<Buffer>,
}

impl VkInterface {
    fn init(window: winit::window::Window) -> Result<VkInterface, Box<dyn std::error::Error>> {
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
        let mut swapchain = Swapchain::init(
            &instance,
            physical_device,
            &device,
            &surface,
            &queue_families,
        )?;
        let renderpass = init_renderpass(&device, physical_device, &surface)?;
        swapchain.create_framebuffers(&device, renderpass)?;
        let pipeline = Pipeline::init(&device, &swapchain, &renderpass)?;
        let pools = Pools::init(&device, &queue_families)?;

        let allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            std::rc::Rc::new(&instance),
            std::rc::Rc::new(&device),
            physical_device,
        );
        let mut allocator = vk_mem::Allocator::new(allocator_create_info)?;
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuToGpu,
            ..Default::default()
        };

        //buffer of vertex positions
        let mut buffer1 = Buffer::new(&allocator, 96, vk::BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu, )?;
        unsafe { buffer1.fill(&allocator,&[
            0.5f32, -0.4f32, 0.0f32, 1.0f32,
            0.0f32, 0.4f32, 0.0f32, 1.0f32,
            -0.5f32, -0.4f32, 0.0f32, 1.0f32,
        ])?};

        //buffer of size and colour
        let mut buffer2 = Buffer::new(&allocator, 60, vk::BufferUsageFlags::VERTEX_BUFFER,
                                  vk_mem::MemoryUsage::CpuToGpu, )?;
        unsafe { buffer2.fill(&allocator, &[
            15.0f32, 1.0f32, 0.0f32, 0.0f32, 1.0f32,
            15.0f32, 0.0f32, 1.0f32, 0.0f32, 1.0f32,
            15.0f32, 0.0f32, 0.0f32, 1.0f32, 1.0f32,
        ])?};

        let command_buffers =
            create_commandbuffers(&device, &pools, swapchain.amount_of_images as usize)?;
        fill_commandbuffers(
            &command_buffers,
            &device,
            &renderpass,
            &swapchain,
            &pipeline,
            &buffer1.buffer,
            &buffer2.buffer,
        )?;

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
            buffers: vec![buffer1, buffer2],
        })
    }
}

//TODO - a semaphore isn't dropped?, also allocs being horrible
impl Drop for VkInterface {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("something wrong while waiting");
            /*for b in &self.buffers {
                self.allocator
                    .destroy_buffer(b.buffer, b.allocation)
                    .expect("problem with buffer destruction");
            }*/
            std::mem::ManuallyDrop::drop(&mut self.allocator);
            self.pools.cleanup(&self.device);
            self.pipeline.cleanup(&self.device);
            self.device.destroy_render_pass(self.renderpass, None);
            self.swapchain.cleanup(&self.device);
            self.device.destroy_device(None);
            std::mem::ManuallyDrop::drop(&mut self.surface);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            self.instance.destroy_instance(None)
        };
    }
}

//to.dos show important notes of things that could be improved
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&eventloop)?;
    let mut vk_struct = VkInterface::init(window)?;
    use winit::event::{Event, WindowEvent};
    eventloop.run(move |event, _, controlflow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        }
        Event::MainEventsCleared => {
            // doing the work here (later)
            vk_struct.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            let (image_index, _) = unsafe {
                vk_struct
                    .swapchain
                    .swapchain_loader
                    .acquire_next_image(
                        vk_struct.swapchain.swapchain,
                        u64::MAX,
                        vk_struct.swapchain.image_available[vk_struct.swapchain.current_image],
                        vk::Fence::null(),
                    )
                    .expect("image acquisition trouble")
            };
            unsafe {
                vk_struct
                    .device
                    .wait_for_fences(
                        &[
                            vk_struct.swapchain.may_begin_drawing
                                [vk_struct.swapchain.current_image],
                        ],
                        true,
                        u64::MAX,
                    )
                    .expect("fence-waiting");
                vk_struct
                    .device
                    .reset_fences(&[
                        vk_struct.swapchain.may_begin_drawing[vk_struct.swapchain.current_image]
                    ])
                    .expect("resetting fences");
            }
            let semaphores_available =
                [vk_struct.swapchain.image_available[vk_struct.swapchain.current_image]];
            let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let semaphores_finished =
                [vk_struct.swapchain.rendering_finished[vk_struct.swapchain.current_image]];
            let commandbuffers = [vk_struct.command_buffers[image_index as usize]];
            let submit_info = [vk::SubmitInfo::builder()
                .wait_semaphores(&semaphores_available)
                .wait_dst_stage_mask(&waiting_stages)
                .command_buffers(&commandbuffers)
                .signal_semaphores(&semaphores_finished)
                .build()];
            unsafe {
                vk_struct
                    .device
                    .queue_submit(
                        vk_struct.queues.graphics_queue,
                        &submit_info,
                        vk_struct.swapchain.may_begin_drawing[vk_struct.swapchain.current_image],
                    )
                    .expect("queue submission");
            };
            let swapchains = [vk_struct.swapchain.swapchain];
            let indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&semaphores_finished)
                .swapchains(&swapchains)
                .image_indices(&indices);
            unsafe {
                vk_struct
                    .swapchain
                    .swapchain_loader
                    .queue_present(vk_struct.queues.graphics_queue, &present_info)
                    .expect("queue presentation");
            };
            vk_struct.swapchain.current_image = (vk_struct.swapchain.current_image + 1)
                % vk_struct.swapchain.amount_of_images as usize;
        }
        _ => {}
    });

    Ok(())
}
