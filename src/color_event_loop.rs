use color_eyre::eyre::Result;
use winit::{
    event::{
        ElementState, Event, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta,
        TouchPhase, VirtualKeyCode, WindowEvent,
    },
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
};

use crate::picker_context::PickerContext;
use crate::Args;

pub fn get_color(args: &Args) -> Result<Option<(u8, u8, u8)>> {
    let mut event_loop: EventLoop<()> = EventLoop::new();

    let mut ctx = PickerContext::new(&event_loop, args)?;

    let mut position = None;

    let mut colors = None;

    let mut mouse_events = 0;

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
                mouse_events += 1;

                position = Some((new_position.cast::<u32>(), window_id));

                if ctx.should_display_zoom() {
                    // Prevents initial incorrect mouse position from sticking to other windows
                    if mouse_events >= 5 {
                        ctx.request_draw_all();
                    } else {
                        ctx.request_draw(window_id);
                    }
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    }
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Return),
                                ..
                            },
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
            } => control_flow.set_exit(),
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(VirtualKeyCode::Z),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                ctx.hold_zoom = match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                };

                if let Some((_, window_id)) = position {
                    ctx.request_draw(window_id);
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Right,
                        ..
                    },
                ..
            } =>
                ctx.hold_right_click = match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                },
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
                event:
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_, vertical_amount),
                        phase: TouchPhase::Moved,
                        ..
                    },
                ..
            } => {
                if ctx.should_display_zoom() {
                    if ctx.hold_right_click {
                        ctx.change_zoom_size(vertical_amount);
                    } else {
                        ctx.change_zoom(vertical_amount);
                    }

                    if let Some((_, window_id)) = position {
                        ctx.request_draw(window_id);
                    }
                }
            }
            Event::RedrawRequested(window_id) => {
                if let Some((pos, cursor_window)) = position {
                    if ctx.should_display_zoom() && cursor_window == window_id {
                        ctx.redraw_window(window_id, pos)
                            .unwrap_or_else(|| ctx.draw_empty_window(window_id));
                    } else {
                        ctx.draw_empty_window(window_id);
                    }
                } else {
                    ctx.draw_empty_window(window_id);
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let (pos, window_id) = match position {
                    Some((ref mut pos, window_id)) => (pos, window_id),
                    _ => return,
                };

                match key {
                    VirtualKeyCode::A | VirtualKeyCode::H | VirtualKeyCode::Left => pos.x -= 1,
                    VirtualKeyCode::S | VirtualKeyCode::J | VirtualKeyCode::Down => pos.y += 1,
                    VirtualKeyCode::W | VirtualKeyCode::K | VirtualKeyCode::Up => pos.y -= 1,
                    VirtualKeyCode::D | VirtualKeyCode::L | VirtualKeyCode::Right => pos.x += 1,
                    _ => return,
                };

                ctx.request_draw(window_id);
            }
            _ => (),
        }
    });

    Ok(colors)
}
