use std::{cmp::Ordering, collections::HashMap};

use color_eyre::eyre::{eyre, Result};
use image::{imageops, DynamicImage, GenericImage, GenericImageView, Pixel, Rgba};
use softbuffer::{GraphicsContext, SoftBufferError};
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoop,
    monitor::{MonitorHandle, VideoMode},
    window::{Fullscreen, Window, WindowBuilder, WindowId, WindowLevel},
};

use crate::args::Args;
use crate::screenshots::screenshots_ordered;

pub struct PickerContext {
    windows: Vec<Window>,
    graphics: HashMap<WindowId, (GraphicsContext, DynamicImage, SoftBufferImage)>,
    cursor: bool,
    pub toggle_zoom: bool,
    pub hold_zoom: bool,
    pub hold_right_click: bool,
    pub zoom: u32,
    pub zoom_size: u32,
}

type SoftBufferImage = Vec<u32>;

impl PickerContext {
    pub fn new(event_loop: &EventLoop<()>, args: &Args) -> Result<Self> {
        let monitors = event_loop.available_monitors().collect::<Vec<_>>();

        let images = screenshots_ordered(&monitors)?;

        let cursor = args.size >= 5;

        let windows = monitors
            .into_iter()
            .map(|monitor| {
                let mut builder = WindowBuilder::new()
                    .with_decorations(false)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_resizable(false)
                    .with_maximized(true);

                if args.exclusive {
                    let video_mode = get_ideal_video_mode(monitor).unwrap();
                    builder = builder.with_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
                } else {
                    builder = builder.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
                }

                let built = builder.build(event_loop)?;

                built.set_cursor_icon(winit::window::CursorIcon::Crosshair);
                built.set_cursor_visible(cursor);
                Ok(built)
            })
            .collect::<Result<Vec<_>>>()?;

        let graphics = windows
            .iter()
            .zip(images)
            .map(|(window, image)| {
                let ctx = unsafe { GraphicsContext::new(&window, &window) }?;
                let cached = image_to_softbuffer(&image);
                Ok((window.id(), (ctx, image, cached)))
            })
            .collect::<Result<_, SoftBufferError>>()
            .map_err(|err| eyre!("Could not create graphics context: {err}"))?;

        Ok(Self {
            windows,
            graphics,
            cursor,
            toggle_zoom: false,
            hold_zoom: args.zoom,
            hold_right_click: false,
            zoom: args.scale.pow(2),
            zoom_size: args.size,
        })
    }

    pub fn set_cursor(&mut self, cursor: bool) {
        if self.zoom_size <= 5 {
            return;
        }

        self.unchecked_cursor_update(cursor);
    }

    fn unchecked_cursor_update(&mut self, cursor: bool) {
        if self.cursor == cursor {
            return;
        }

        self.cursor = cursor;
        self.windows
            .iter_mut()
            .for_each(|x| x.set_cursor_visible(cursor))
    }

    pub fn get_pixel(
        &self,
        window_id: &WindowId,
        position: PhysicalPosition<u32>,
    ) -> Option<(u8, u8, u8)> {
        let (_, image, _) = self.graphics.get(window_id)?;
        let pixel = image.get_pixel(position.x, position.y).0;

        Some((pixel[0], pixel[1], pixel[2]))
    }

    pub fn draw_empty_window(&mut self, window_id: WindowId) {
        let (graphics_ctx, ref image, ref cached) = self.graphics.get_mut(&window_id).unwrap();

        graphics_ctx.set_buffer(cached, image.width() as u16, image.height() as u16);
    }

    pub fn redraw_window(
        &mut self,
        window_id: WindowId,
        mouse_pos: PhysicalPosition<u32>,
    ) -> Option<()> {
        let (graphics_ctx, ref image, _) = self.graphics.get_mut(&window_id).unwrap();

        let square_halfway: u32 = self.zoom_size / 2;

        let zoom_start_in_bounds = image.in_bounds(
            mouse_pos.x.checked_sub(square_halfway)?,
            mouse_pos.y.checked_sub(square_halfway)?,
        );

        let zoom_end_in_bounds =
            image.in_bounds(mouse_pos.x + square_halfway, mouse_pos.y + square_halfway);

        if !zoom_start_in_bounds | !zoom_end_in_bounds {
            return None;
        }

        let mut image = image.clone();

        // An additional line for the final x and y grid lines
        let zoomed_size: u32 = self.zoom_size * self.zoom + 1;
        let zoom_halfway: u32 = zoomed_size / 2;

        let cropped_image = image.crop_imm(
            mouse_pos.x - square_halfway,
            mouse_pos.y - square_halfway,
            self.zoom_size,
            self.zoom_size,
        );

        let mut total_light_value = 0;

        for (_, _, pixel) in cropped_image.pixels() {
            let grayscale = pixel.to_luma();

            total_light_value += grayscale.0[0] as u32;
        }

        let average_light_value = total_light_value / (self.zoom_size.pow(2));
        let border_color = (255 - average_light_value) as u8;

        let mut zoomed_in_image =
            cropped_image.resize(zoomed_size, zoomed_size, imageops::FilterType::Nearest);

        let zoom = self.zoom as usize;
        let border_color = Rgba([border_color, border_color, border_color, 255]);

        for new_line in (0..zoomed_size).step_by(zoom) {
            for line_length in 0..zoomed_size {
                zoomed_in_image.put_pixel(new_line, line_length, border_color);
                zoomed_in_image.put_pixel(line_length, new_line, border_color);
            }
        }

        imageops::replace(
            &mut image,
            &zoomed_in_image,
            (mouse_pos.x as i64) - zoom_halfway as i64,
            (mouse_pos.y as i64) - zoom_halfway as i64,
        );

        let buffer = image_to_softbuffer(&image);

        graphics_ctx.set_buffer(&buffer, image.width() as u16, image.height() as u16);

        Some(())
    }

    pub fn request_draw_all(&self) {
        self.windows
            .iter()
            .for_each(|window| window.request_redraw());
    }

    pub fn request_draw(&self, window_id: WindowId) {
        self.windows
            .iter()
            .find(|x| x.id() == window_id)
            .unwrap()
            .request_redraw();
    }

    pub const fn should_display_zoom(&self) -> bool {
        self.toggle_zoom ^ self.hold_zoom
    }

    pub fn change_zoom(&mut self, change_in_zoom: f32) {
        let is_neg = change_in_zoom.is_sign_negative();
        let change_in_zoom = (change_in_zoom as i32).unsigned_abs() * 2;

        if is_neg {
            self.zoom = self.zoom.saturating_div(change_in_zoom).max(2);
        } else {
            self.zoom = self.zoom.saturating_mul(change_in_zoom).min(256);
        }
    }

    pub fn change_zoom_size(&mut self, change_in_size: f32) {
        let is_neg = change_in_size.is_sign_negative();
        let change_in_size = (change_in_size as i32).unsigned_abs() * 2;

        if is_neg {
            self.zoom_size = self.zoom_size.saturating_sub(change_in_size).max(1);
            if self.zoom_size <= 5 {
                self.unchecked_cursor_update(false);
            }
        } else {
            self.zoom_size += change_in_size;
            if self.zoom_size > 5 {
                self.unchecked_cursor_update(true);
            }
        }
    }
}

fn get_ideal_video_mode(monitor: MonitorHandle) -> Option<VideoMode> {
    monitor.video_modes().reduce(|prev, current| {
        let size: (u32, u32) = current.size().into();
        let other_size: (u32, u32) = prev.size().into();

        match size.cmp(&other_size).then(
            current.bit_depth().cmp(&prev.bit_depth()).then(
                current
                    .refresh_rate_millihertz()
                    .cmp(&prev.refresh_rate_millihertz()),
            ),
        ) {
            Ordering::Greater | Ordering::Equal => current,
            Ordering::Less => prev,
        }
    })
}

fn image_to_softbuffer(image: &DynamicImage) -> SoftBufferImage {
    let buffer = image
        .as_rgba8()
        .map(|image| image.chunks(4))
        // SAFETY: `screenshots` crate and flameshot should both be returning either RGBA8 or RGB8
        .unwrap_or_else(|| image.as_rgb8().unwrap().chunks(3));

    let buffer: SoftBufferImage = buffer
        .map(|rgb| rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16))
        .collect();

    buffer
}
