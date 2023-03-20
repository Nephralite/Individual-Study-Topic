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
        vec![ash::extensions::ext::DebugUtils::name().as_ptr()];
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .p_application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);
    //having finally set all our variables, make an instance from them
    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

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
    Ok(())
}
fn main_loop() {}

fn cleanup(instance: ash::Instance) {
    //yeet the vulkan instance and its messenger
    unsafe { instance.destroy_instance(None) };
    debug_utils.destroy_debug_utils_messenger(utils_messenger, None);
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
