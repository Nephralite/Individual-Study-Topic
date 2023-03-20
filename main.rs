use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}

//TODO: fix dropping of variables between functions
fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_window()?;
    init_vulkan()?;
    main_loop();
    cleanup(instance);
    Ok(())
}

fn init_window() -> Result<(), Box<dyn std::error::Error>> {
    //create a window with the magic of winit
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop)?;
    Ok(())
}

fn init_vulkan() -> Result<(), Box<dyn std::error::Error>> {
    //initializing vulkan, requires a lot of variables to get debug info
    let entry = ash::Entry::new();
    //create the info
    let app_name = std::ffi::CString::new("Jades Black Window").unwrap();
    let engine_name = std::ffi::CString::new("UnknownEngine").unwrap(); //technically optional but kept freaking out when unassigned
    let app_info = vk::ApplicationInfo::builder() //give vulkan basic information about our app, using the ash builder
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 0, 1))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0)) //a non existent but mandatory version no
        .api_version(vk::make_api_version(0, 1, 0, 106));
    let layer_names: Vec<std::ffi::CString> = //enable validation layers so we get errors, which are off by default
        vec![std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layer_name_pointers: Vec<*const i8> = layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let extension_name_pointers: Vec<*const i8> =
        vec![
            ash::extensions::ext::DebugUtils::name().as_ptr(),
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::XlibSurface::name().as_ptr(),
        ];
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .p_application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);
    //having finally set all our variables, make an instance from them
    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    //attach vulkan to our window by abusing the power of linux x11
    use winit::platform::unix::WindowExtUnix;
    let x11_display = window.xlib_display().unwrap();
    let x11_window = window.xlib_window().unwrap();
    let x11_create_info = vk::XlibSurfaceCreateInfoKHR::builder()
        .window(x11_window)
        .dpy(x11_display as *mut vk::Display);
    let xlib_surface_loader = ash::extensions::khr::XlibSurface::new(&entry, &instance);
    let surface = unsafe { xlib_surface_loader.create_xlib_surface(&x11_create_info, None) }?;
    let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

    //make a messenger for that instance
    let debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);
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
    let utils_messenger =
        unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None)? };



    //fetch a graphics card, which vulkan calls physical devices
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    let (physical_device, device_properties) = {
        let mut chosen = None;
        for p in phys_devs {
            let properties = unsafe { instance.get_physical_device_properties(p) };
            if chosen == None {
                //grab the first device just in case
                chosen = Some((p, properties));
            } else if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                //grab a desktop GPU where possible
                chosen = Some((p, properties));
            }
            chosen.unwrap()
        }
    };

    //select some queues to send the data to
    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
    let qfam_indices = {
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, qfam) in queue_family_properties.iter().enumerate() {
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && unsafe { //TODO: check if the case where the graphics can not draw on the surface is a problem
                surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, surface)?
            }
            {
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
        (
            found_graphics_q_index.unwrap(),
            found_transfer_q_index.unwrap(),
        )
    };
    let priorities = [1.0f32];
    let queue_infos = [
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(qfam_indices.0)
            .queue_priorities(&priorities)
            .build(),
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(qfam_indices.1)
            .queue_priorities(&priorities)
            .build(),
    ];
    //use queue to make logical device
    let device_extension_name_pointers: Vec<*const i8> =
        vec![ash::extensions::khr::Swapchain::name().as_ptr()];
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let graphics_queue = unsafe { logical_device.get_device_queue(qfamindices.0, 0) };
    let transfer_queue = unsafe { logical_device.get_device_queue(qfamindices.1, 0) };

    //make swapchains
    let surface_capabilities = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    };
    let surface_present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    };
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) };
    let queue_families = [qfamindices.0];
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(
            3.max(surface_capabilities.min_image_count)
                .min(surface_capabilities.max_image_count),
        )
        .image_format(surface_formats.first().unwrap().format)
        .image_color_space(surface_formats.first().unwrap().color_space)
        .image_extent(surface_capabilities.current_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&queue_families)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO);
    let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
    let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };


    Ok(())
}
fn main_loop() {}

fn cleanup(instance: ash::Instance) {
    //yeet the vulkan instance and its messenger
    unsafe {
        logical_device.destroy_device(None);
        surface_loader.destroy_surface(surface, None);
        debug_utils.destroy_debug_utils_messenger(utils_messenger, None);
        instance.destroy_instance(None);
    };
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    //converts errors to readable format
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
