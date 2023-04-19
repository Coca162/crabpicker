use image::{GenericImageView, ImageFormat, DynamicImage};
use screenshots::Screen;
use softbuffer::GraphicsContext;
use std::collections::HashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};

#[allow(clippy::must_use_candidate)]
pub fn get_color(event_loop: &mut EventLoop<()>) -> Option<(u8, u8, u8)> {
    let (mut windows, mut map) = monitor_windows_with_screenshots(event_loop)?;

    let mut position = None;

    let mut colors = None;

    event_loop.run_return(|event, _, control_flow| {
        control_flow.set_wait();

        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CursorMoved {
                        position: new_position,
                        ..
                    },
                ..
            } => position = Some(new_position),
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    },
                window_id,
            } => {
                let position = if let Some(pos) = position {
                    pos.cast::<u32>()
                } else {
                    control_flow.set_exit();
                    return;
                };

                println!("{position:?}");

                let (_, image) = &map[&window_id];
                colors = Some(get_pixel(image, position));

                windows.clear();

                control_flow.set_exit();
            }
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                windows.clear();
            }
            Event::RedrawRequested(window_id) => {
                let (ctx, image) = map.get_mut(&window_id).unwrap();

                let buffer: Vec<u32> = image.as_rgba8().unwrap().chunks(4).map(|rgb|
                    rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16)
                ).collect();
                
                ctx.set_buffer(&buffer, image.width() as u16, image.height() as u16);
            }
            _ => (),
        }
    });
    colors
}

fn monitor_windows_with_screenshots(
    event_loop: &EventLoop<()>,
) -> Option<(Vec<Window>, HashMap<WindowId, (GraphicsContext, DynamicImage)>)> {
    let screens = Screen::all().ok()?;

    let monitors: HashMap<_, _> = event_loop
        .available_monitors()
        .map(|monitor| (monitor.position(), monitor))
        .collect();

    let windows: Vec<Window> = screens
        .iter()
        .map(|screen| screen.display_info)
        .map(|info| PhysicalPosition::new(info.x, info.y))
        .map(|position| monitors[&position].clone())
        .map(|monitor| Fullscreen::Borderless(Some(monitor)))
        .map(|fullscreen| WindowBuilder::new().with_fullscreen(Some(fullscreen)))
        .map(|builder| builder.build(event_loop).ok())
        .collect::<Option<_>>()?;

    let images: Vec<DynamicImage> = screens
        .iter()
        .map(|screen| screen.capture().ok().unwrap())
        .map(|image| image::load_from_memory_with_format(image.buffer(), ImageFormat::Png).unwrap())
        .collect();

    let map = windows
        .iter()
        .zip(images)
        .map(|(window, image)| {
            (
                window.id(),
                (
                    unsafe { GraphicsContext::new(&window, &window) }.unwrap(),
                    image,
                ),
            )
        })
        .collect();

    Some((windows, map))
}

fn get_pixel(image: &DynamicImage, position: PhysicalPosition<u32>) -> (u8, u8, u8) {
    let pixel = image.get_pixel(position.x, position.y).0;

    (pixel[0], pixel[1], pixel[2])
}
