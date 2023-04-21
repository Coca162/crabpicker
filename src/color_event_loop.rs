use color_eyre::eyre::Result;
use winit::{
    event::{
        ElementState, Event, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode,
        WindowEvent, MouseScrollDelta, TouchPhase,
    },
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
            } => {
                position = Some((new_position.cast::<u32>(), window_id));
                ctx.request_draw(window_id);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                window_id,
            } => {
                position = None;
                ctx.request_draw(window_id);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                if let Some(new_colors) = position.and_then(|(pos, id)| ctx.get_pixel(&id, pos)) {
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
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Z),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                ctx.hold_zoom = true;
                
                if let Some((_, window_id)) = position {
                    ctx.request_draw(window_id);
                }            
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Z),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                ctx.hold_zoom = false;

                if let Some((_, window_id)) = position {
                    ctx.request_draw(window_id);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(ModifiersState::CTRL),
                ..
            } => {
                ctx.toggle_zoom = !ctx.toggle_zoom;

                if let Some((_, window_id)) = position {
                    ctx.request_draw(window_id);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel {delta: MouseScrollDelta::LineDelta(_, vertical_amount), phase: TouchPhase::Moved, .. }, ..
            } => {
                ctx.change_zoom(vertical_amount);

                if let Some((_, window_id)) = position {
                    ctx.request_draw(window_id);
                }
            }
            Event::RedrawRequested(window_id) => {
                if let Some((pos, cursor_window)) = position {
                    if ctx.should_display_zoom() && cursor_window == window_id {
                        ctx.redraw_window(window_id, pos).unwrap_or_else(|| ctx.draw_empty_window(window_id));
                    } else {
                        ctx.draw_empty_window(window_id);
                    }
                } else {
                    ctx.draw_empty_window(window_id);
                }
            }
            _ => (),
        }
    });

    Ok(colors)
}
