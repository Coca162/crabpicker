use std::collections::HashMap;

use color_eyre::{
    eyre::{eyre, Result},
    Report,
};
use image::{imageops, DynamicImage, GenericImage, GenericImageView, Pixel, Rgba};
use softbuffer::{GraphicsContext, SoftBufferError};
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder, WindowId, WindowLevel},
};

use crate::screenshots::screenshots_ordered;

pub struct PickerContext {
    windows: Vec<Window>,
    graphics: HashMap<WindowId, (GraphicsContext, DynamicImage)>,
    pub toggle_zoom: bool,
    pub hold_zoom: bool,
    pub zoom: u32
}

impl PickerContext {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let monitors = event_loop.available_monitors().collect::<Vec<_>>();

        let images = screenshots_ordered(&monitors)?;

        let windows = monitors
            .into_iter()
            .map(|monitor| {
                let build = WindowBuilder::new()
                    .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))))
                    .with_decorations(false)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .build(event_loop)
                    .map_err(Into::<Report>::into)?;

                build.set_cursor_icon(winit::window::CursorIcon::Crosshair);
                Ok(build)
            })
            .collect::<Result<Vec<_>>>()?;

        let graphics = windows
            .iter()
            .zip(images)
            .map(|(window, image)| {
                let ctx = unsafe { GraphicsContext::new(&window, &window) }?;
                Ok((window.id(), (ctx, image)))
            })
            .collect::<Result<_, SoftBufferError>>()
            .map_err(|err| eyre!("Could not create graphics context: {err}"))?;

        Ok(Self {
            windows,
            graphics,
            toggle_zoom: false,
            hold_zoom: false,
            zoom: 16
        })
    }

    pub fn get_pixel(
        &self,
        window_id: &WindowId,
        position: PhysicalPosition<u32>,
    ) -> Option<(u8, u8, u8)> {
        let (_, image) = self.graphics.get(window_id)?;
        let pixel = image.get_pixel(position.x, position.y).0;

        Some((pixel[0], pixel[1], pixel[2]))
    }

    pub fn draw_empty_window(
        &mut self,
        window_id: WindowId
    ) {
        let (graphics_ctx, ref image) = self.graphics.get_mut(&window_id).unwrap();

        save_image_to_graphics_buffer(graphics_ctx, image);
    }

    pub fn redraw_window(
        &mut self,
        window_id: WindowId,
        mouse_pos: PhysicalPosition<u32>,
    ) -> Option<()> {
        let (graphics_ctx, ref image) = self.graphics.get_mut(&window_id).unwrap();

        if !image.in_bounds(mouse_pos.x.checked_sub(SQUARE_HALFWAY)?, mouse_pos.y.checked_sub(SQUARE_HALFWAY)?) {
            return None;
        }

        if !image.in_bounds(mouse_pos.x + SQUARE_HALFWAY, mouse_pos.y + SQUARE_HALFWAY) {
            return None;
        }

        let mut image = image.clone();

        const SQUARE_SIZE: u32 = 11;
        const SQUARE_HALFWAY: u32 = SQUARE_SIZE / 2;
        // An additional line for the final x and y grid lines
        let zoomed_size: u32 = SQUARE_SIZE * self.zoom + 1;
        let zoom_halfway: u32 = zoomed_size / 2;

        let cropped_image = image.crop_imm(mouse_pos.x - SQUARE_HALFWAY, mouse_pos.y - SQUARE_HALFWAY, SQUARE_SIZE, SQUARE_SIZE);

        let mut total_light_value = 0;

        for (_, _, pixel) in cropped_image.pixels() {
            let grayscale = pixel.to_luma();

            total_light_value += grayscale.0[0] as u32;
        }

        let average_light_value = total_light_value / (SQUARE_SIZE.pow(2));
        let border_color = (255 - average_light_value) as u8;

        let mut zoomed_in_image = cropped_image.resize(zoomed_size, zoomed_size, imageops::FilterType::Nearest);

        draw_grid(
            (zoomed_size, zoomed_size),
            &mut zoomed_in_image,
            border_color,
            self.zoom as usize
        );

        imageops::replace(
            &mut image,
            &zoomed_in_image,
            (mouse_pos.x as i64) - zoom_halfway as i64,
            (mouse_pos.y as i64) - zoom_halfway as i64,
        );


        save_image_to_graphics_buffer(graphics_ctx, &image);

        Some(())
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
        if !self.should_display_zoom() {
            return;
        }

        let change_in_zoom = (change_in_zoom as i32) * 2;
        if change_in_zoom.is_negative() {
            if self.zoom != 4 {
                self.zoom /= change_in_zoom.unsigned_abs();
            }
        } else if self.zoom != 64 {
            self.zoom *= change_in_zoom.unsigned_abs();
        }
    }
}

fn save_image_to_graphics_buffer(graphics_ctx: &mut GraphicsContext, image: &DynamicImage) {
    let buffer = image
        .as_rgba8()
        .map(|image| image.chunks(4))
        // SAFETY: `screenshots` crate and flameshot should both be returning either RGBA8 or RGB
        .unwrap_or_else(|| image.as_rgb8().unwrap().chunks(3));

    let buffer: Vec<u32> = buffer
        .map(|rgb| rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16))
        .collect();

    graphics_ctx.set_buffer(&buffer, image.width() as u16, image.height() as u16);
}

fn draw_grid((width, height): (u32, u32), color_image: &mut DynamicImage, color: u8, zoom: usize) {
    let color = Rgba([color, color, color, 255]);

    // We need two for loops because the crop may not always be a square
    for x in (0..width).step_by(zoom) {
        for y in 0..height {
            color_image.put_pixel(x, y, color);
        }
    }

    for y in (0..height).step_by(zoom) {
        for x in 0..width {
            color_image.put_pixel(x, y, color);
        }
    }
}
