use color_eyre::eyre::Result;
use winit::{
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
};

use crate::picker_context::PickerContext;

pub fn get_color(event_loop: &mut EventLoop<()>) -> Result<Option<(u8, u8, u8)>> {
    let mut ctx = PickerContext::new(event_loop)?;

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
                window_id,
            } => position = Some((new_position, window_id)),
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                if let Some(new_colors) = position
                    .map(|(pos, id)| (pos.cast::<u32>(), id))
                    .and_then(|(pos, id)| ctx.get_pixel(&id, pos))
                {
                    control_flow.set_exit();
                    colors = Some(new_colors);
                }
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
                control_flow.set_exit();
            }
            Event::RedrawRequested(window_id) => {
                ctx.redraw_window(&window_id, position);
            }
            _ => (),
        }
    });

    Ok(colors)
}
