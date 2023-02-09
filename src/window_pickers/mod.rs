use iced_winit::winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};
use image::{GenericImageView, ImageFormat};
use screenshots::{Image, Screen};
use std::collections::HashMap;

#[allow(clippy::must_use_candidate)]
pub fn get_color(event_loop: &mut EventLoop<()>) -> Option<(u8, u8, u8)> {
    let (_windows, map) = monitor_windows_with_screenshots(event_loop)?;

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

                let image = &map[&window_id];
                colors = Some(get_pixel(image, position));

                control_flow.set_exit();
            }
            _ => (),
        }
    });

    colors
}

fn monitor_windows_with_screenshots(
    event_loop: &EventLoop<()>,
) -> Option<(Vec<Window>, HashMap<WindowId, Image>)> {
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

    let images: Vec<Image> = screens
        .iter()
        .map(|x| x.capture().ok())
        .collect::<Option<_>>()?;

    let map = windows.iter().map(Window::id).zip(images).collect();

    Some((windows, map))
}

fn get_pixel(image: &Image, position: PhysicalPosition<u32>) -> (u8, u8, u8) {
    let image = image::load_from_memory_with_format(image.buffer(), ImageFormat::Png).unwrap();

    let pixel = image.get_pixel(position.x, position.y).0;

    (pixel[0], pixel[1], pixel[2])
}
