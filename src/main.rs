use ash::vk;
use nalgebra as na;
use crate::camera::Camera;
use crate::model::{Model, InstanceData};
use crate::swapchain::Swapchain;
use crate::vkinterface::VkInterface;

mod camera;
mod buffer;
mod debug;
mod model;
mod commandbuffers;
mod rendering;
mod surface;
mod swapchain;
mod vkinterface;
mod initialization;

//to.dos show important notes of things that could be improved
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&eventloop)?;
    let mut vk_struct = VkInterface::init(window)?;

    let mut cube = Model::cube();
    let mut object = Model::object("squirrel.obj");
    object.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.1, 0.2, 0.4))
            * na::Matrix4::new_scaling(0.01))
            .into(),
        colour: [0.0, 0.0, 1.0],
    });
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.1))
            * na::Matrix4::new_scaling(0.1))
            .into(),
        colour: [0.2, 0.4, 1.0],
    });
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.05, 0.05, 0.0))
            * na::Matrix4::new_scaling(0.1))
            .into(),
        colour: [1.0, 1.0, 0.2],
    });
    for i in 0..10 {
        for j in 0..10 {
            cube.insert_visibly(InstanceData {
                modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(
                    i as f32 * 0.2 - 1.0,
                    j as f32 * 0.2 - 1.0,
                    0.5,
                )) * na::Matrix4::new_scaling(0.03))
                    .into(),
                colour: [1.0, i as f32 * 0.07, j as f32 * 0.07],
            });
            cube.insert_visibly(InstanceData {
                modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(
                    i as f32 * 0.2 - 1.0,
                    0.0,
                    j as f32 * 0.2 - 1.0,
                )) * na::Matrix4::new_scaling(0.02))
                    .into(),
                colour: [i as f32 * 0.07, j as f32 * 0.07, 1.0],
            });
        }
    }
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::from_scaled_axis(na::Vector3::new(0.0, 0.0, 1.4))
            * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.5, 0.0))
            * na::Matrix4::new_scaling(0.1))
            .into(),
        colour: [0.0, 0.5, 0.0],
    });
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.5, 0.0, 0.0))
            * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.5, 0.01, 0.01)))
            .into(),
        colour: [1.0, 0.5, 0.5],
    });
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.5, 0.0))
            * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.01, 0.5, 0.01)))
            .into(),
        colour: [0.5, 1.0, 0.5],
    });
    cube.insert_visibly(InstanceData {
        modelmatrix: (na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0))
            * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.01, 0.01, 0.5)))
            .into(),
        colour: [0.5, 0.5, 1.0],
    });
    cube.update_vertexbuffer(&vk_struct.allocator).unwrap();
    cube.update_instancebuffer(&vk_struct.allocator).unwrap();
    vk_struct.models = vec![cube];

    let mut camera = Camera::default();


    use winit::event::{Event, WindowEvent};
    eventloop.run(move |event, _, controlflow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            if let winit::event::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: Some(keycode),
                ..
            } = input
            {
                match keycode {
                    winit::event::VirtualKeyCode::Right => {
                        camera.turn_right(0.1);
                    }
                    winit::event::VirtualKeyCode::Left => {
                        camera.turn_left(0.1);
                    }
                    winit::event::VirtualKeyCode::Up => {
                        camera.move_forward(0.05);
                    }
                    winit::event::VirtualKeyCode::Down => {
                        camera.move_backward(0.05);
                    }
                    winit::event::VirtualKeyCode::PageUp => {
                        camera.turn_up(0.02);
                    }
                    winit::event::VirtualKeyCode::PageDown => {
                        camera.turn_down(0.02);
                    }
                    _ => {}
                }
            }
        }
        Event::MainEventsCleared => {

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
            camera.update_buffer(&vk_struct.allocator, &mut vk_struct.uniformbuffer);
            for m in &mut vk_struct.models {
                m.update_instancebuffer(&vk_struct.allocator).unwrap();
            }
            vk_struct
                .update_commandbuffer(image_index as usize)
                .expect("updating the command buffer");

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
}
