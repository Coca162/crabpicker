use copypasta_ext::{x11_bin::ClipboardContext, prelude::ClipboardProvider};
use image::{io::Reader, GenericImageView, ImageFormat};
use screenshots::{Image, Screen};
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};

fn main() {
    let event_loop = EventLoop::new();

    let (_windows, map) = monitor_windows_with_screenshots(&event_loop);

    let mut position = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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
                let position = position.unwrap().cast::<u32>();
                println!("{position:?}");

                let image = &map[&window_id];
                let (r, g, b) = get_pixel(image, position);

                let rgb_hex = format!("#{r:02X}{g:02X}{b:02X}");

                print_result((r, g, b), &rgb_hex);

                ClipboardContext::new().unwrap().set_contents(rgb_hex).unwrap();
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}

fn monitor_windows_with_screenshots(
    event_loop: &EventLoop<()>,
) -> (Vec<Window>, HashMap<WindowId, Image>) {
    let monitors: Vec<_> = event_loop.available_monitors().collect();
    let screens = Screen::all().unwrap();
    let images = screens.iter().map(|x| x.capture().unwrap());

    let window_build = WindowBuilder::new().with_transparent(true);

    let windows: Vec<Window> = screens
        .iter()
        .map(|x| x.display_info)
        .map(|info| PhysicalPosition::new(info.x, info.y))
        .map(|position| monitors.iter().find(|x| position == x.position()).unwrap())
        .cloned()
        .map(|monitor| {
            window_build
                .clone()
                .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))))
        })
        .map(|builder| builder.build(&event_loop).unwrap())
        .collect();

    let map = windows.iter().map(|x| x.id()).zip(images).collect();

    (windows, map)
}

fn get_pixel(image: &Image, position: PhysicalPosition<u32>) -> (u8, u8, u8) {
    let stream = Cursor::new(image.buffer());

    let image = Reader::with_format(stream, ImageFormat::Png)
        .decode()
        .unwrap();

    let pixel = image.get_pixel(position.x, position.y).0;

    (pixel[0], pixel[1], pixel[2])
}

fn print_result((r, g, b): (u8, u8, u8), rgb_hex: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout
        .set_color(
            ColorSpec::new()
                .set_bg(Some(Color::Rgb(r, g, b)))
                .set_fg(Some(Color::Rgb(255 - r, 255 - g, 255 - b))),
        )
        .unwrap();

    stdout.write(rgb_hex.as_bytes()).unwrap();

    stdout.reset().unwrap();

    stdout.flush().unwrap();
}
