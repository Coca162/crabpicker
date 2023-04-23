use std::{cmp::Ordering, collections::HashMap};

use color_eyre::{
    eyre::{eyre, Result},
    Report,
};
use image::{imageops, DynamicImage, GenericImage, GenericImageView, Pixel, Rgba};
use softbuffer::{GraphicsContext, SoftBufferError};
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder, WindowId, WindowLevel}, monitor::{VideoMode, MonitorHandle},
};

use crate::screenshots::screenshots_ordered;
use crate::Args;

pub struct PickerContext {
    windows: Vec<Window>,
    graphics: HashMap<WindowId, (GraphicsContext, DynamicImage, CachedSoftBufferImage)>,
    pub toggle_zoom: bool,
    pub hold_zoom: bool,
    pub hold_right_click: bool,
    pub zoom: u32,
    pub zoom_size: u32,
}

type CachedSoftBufferImage = Vec<u32>;

impl PickerContext {
    pub fn new(event_loop: &EventLoop<()>, args: &Args) -> Result<Self> {
        let monitors = event_loop.available_monitors().collect::<Vec<_>>();

        let images = screenshots_ordered(&monitors)?;

        let windows = monitors
            .into_iter()
            .map(|monitor| {

                let mut builder = WindowBuilder::new()
                    .with_decorations(false)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_resizable(false)
                    .with_maximized(true);

                if args.exclusive_fullscreen {
                    let video_mode = get_ideal_video_mode(monitor);
                    builder = builder.with_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
                } else {
                    builder = builder.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
                }

                let built = builder
                    .build(event_loop)
                    .map_err(Into::<Report>::into)?;

                built.set_cursor_icon(winit::window::CursorIcon::Crosshair);
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
            toggle_zoom: false,
            hold_zoom: args.zoom,
            hold_right_click: true,
            zoom: args.scale.pow(2),
            zoom_size: args.zoom_size,
        })
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

        // // Sometimes the middle pixel is not the one which gets picked for some reason
        // // we use this as a precaution to guarantee the picked pixel is always the one being shown
        // let middle_pixel = image.get_pixel(mouse_pos.x, mouse_pos.y);

        if !image.in_bounds(
            mouse_pos.x.checked_sub(square_halfway)?,
            mouse_pos.y.checked_sub(square_halfway)?,
        ) {
            return None;
        }

        if !image.in_bounds(mouse_pos.x + square_halfway, mouse_pos.y + square_halfway) {
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

        // cropped_image.put_pixel(SQUARE_HALFWAY, SQUARE_HALFWAY, middle_pixel);

        let mut total_light_value = 0;

        for (_, _, pixel) in cropped_image.pixels() {
            let grayscale = pixel.to_luma();

            total_light_value += grayscale.0[0] as u32;
        }

        let average_light_value = total_light_value / (self.zoom_size.pow(2));
        let border_color = (255 - average_light_value) as u8;

        let mut zoomed_in_image =
            cropped_image.resize(zoomed_size, zoomed_size, imageops::FilterType::Nearest);

        draw_grid(
            (zoomed_size, zoomed_size),
            &mut zoomed_in_image,
            border_color,
            self.zoom as usize,
        );

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
        let change_in_zoom = (change_in_zoom as i32) * 2;
        if change_in_zoom.is_negative() {
            if self.zoom != 2 {
                self.zoom /= change_in_zoom.unsigned_abs();
            }
        } else if self.zoom != 256 {
            self.zoom *= change_in_zoom.unsigned_abs();
        }
    }

    pub fn change_zoom_size(&mut self, change_in_size: f32) {
        let change_in_size = (change_in_size as i32) * 2;

        if change_in_size.is_negative() {
            self.zoom_size = self.zoom_size.saturating_sub(change_in_size.unsigned_abs());
        } else {
            self.zoom_size += change_in_size.unsigned_abs();
        }
    }
}

fn get_ideal_video_mode(monitor: MonitorHandle) -> VideoMode {
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
            Ordering::Greater => current,
            Ordering::Equal => current,
            Ordering::Less => prev,
        }
    }).unwrap()
    // let pos = monitor.position();
    // let (x, y) = (pos.x, pos.y);
    // println!("Ideal video mode for {}: {video_mode}", monitor.name().unwrap_or(format!("({x}x{y})")));
}

fn image_to_softbuffer(image: &DynamicImage) -> CachedSoftBufferImage {
    let buffer = image
        .as_rgba8()
        .map(|image| image.chunks(4))
        // SAFETY: `screenshots` crate and flameshot should both be returning either RGBA8 or RGB
        .unwrap_or_else(|| image.as_rgb8().unwrap().chunks(3));

    let buffer: CachedSoftBufferImage = buffer
        .map(|rgb| rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16))
        .collect();

    buffer
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
