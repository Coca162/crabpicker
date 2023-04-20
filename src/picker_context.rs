use std::{collections::HashMap, iter};

use color_eyre::{
    eyre::{eyre, Result},
    Report,
};
use image::{imageops, DynamicImage, GenericImage, GenericImageView, Pixel, Rgba};
use softbuffer::{GraphicsContext, SoftBufferError};
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};

use crate::screenshots::screenshots_ordered;

pub struct PickerContext {
    windows: Vec<Window>,
    graphics: HashMap<WindowId, (GraphicsContext, DynamicImage)>,
    pub ctrl_pressed: bool,
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
            ctrl_pressed: false,
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

    pub fn redraw_window(
        &mut self,
        window_id: &WindowId,
        position: Option<(PhysicalPosition<u32>, WindowId)>,
    ) -> Option<()> {
        let (graphics_ctx, ref image) = self.graphics.get_mut(window_id)?;

        let mut possible_image;

        let final_image = if let Some((pos, ref mouse_window)) = position {
            if mouse_window == window_id && self.ctrl_pressed {
                possible_image = image.clone();
                // // SAFETY: This ignored bound checking because the image should be as big
                // // as the window is, also the bound checking on `get_pixel` is broken lol
                // // https://github.com/image-rs/image/pull/1910
                // let pixel = unsafe { possible_image.unsafe_get_pixel(pos.x, pos.y).0 };

                // let width = 100;
                // let height = 100;
                // let color: Rgba<u8> = Rgba(pixel); // Red color

                // let mut square = generate_picker_square(color);

                // let color_image =
                //     ImageBuffer::from_fn(width, height, |_x, _y| square.next().unwrap());

                let color_image = image.crop_imm(pos.x - 5, pos.y - 5, 11, 11);

                let mut total_light_value = 0;
                let num_pixels = 11 * 11;

                for (_, _, pixel) in color_image.pixels() {
                    let grayscale = pixel.to_luma();

                    total_light_value += grayscale.0[0] as u32;
                }

                let average_light_value = (255 - (total_light_value / num_pixels)) as u8;

                // unsafe { color_image.unsafe_put_pixel(0, 0, Rgba([0, 0, 0, 0])) };
                // unsafe { color_image.unsafe_put_pixel(1, 0, Rgba([0, 0, 0, 0])) };
                // unsafe { color_image.unsafe_put_pixel(0, 1, Rgba([0, 0, 0, 0])) };

                let mut color_image = color_image.resize(177, 177, imageops::FilterType::Nearest);

                let (width, height) = color_image.dimensions();

                // Draw the grid
                for x in 0..width {
                    if x % 16 == 0 {
                        for y in 0..height {
                            color_image.put_pixel(
                                x,
                                y,
                                Rgba([
                                    average_light_value,
                                    average_light_value,
                                    average_light_value,
                                    255,
                                ]),
                            );
                        }
                    }
                }
                for y in 0..height {
                    if y % 16 == 0 {
                        for x in 0..width {
                            color_image.put_pixel(
                                x,
                                y,
                                Rgba([
                                    average_light_value,
                                    average_light_value,
                                    average_light_value,
                                    255,
                                ]),
                            );
                        }
                    }
                }

                imageops::overlay(
                    &mut possible_image,
                    &color_image,
                    pos.x as i64 - 88,
                    pos.y as i64 - 88,
                );

                &possible_image
            } else {
                image
            }
        } else {
            image
        };

        // SAFETY: `screenshots` crate should be returning RGBA8 screenshots
        #[cfg(feature = "screenshots_crate")]
        let buffer = final_image.as_rgba8().unwrap().chunks(4);

        // Flameshot does not provide a reliably8 RGB or RGBA8
        #[cfg(feature = "flameshot")]
        let buffer = final_image.to_rgba8().chunks(4);

        let buffer: Vec<u32> = buffer
            .map(|rgb| rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16))
            .collect();

        graphics_ctx.set_buffer(&buffer, image.width() as u16, image.height() as u16);

        Some(())
    }

    pub fn request_draw(&self, window_id: WindowId) {
        self.windows
            .iter()
            .find(|x| x.id() == window_id)
            .unwrap()
            .request_redraw();
    }

    pub fn in_bounds(&self, (pos, window_id): &(PhysicalPosition<u32>, WindowId)) -> bool {
        self.graphics[window_id].1.in_bounds(pos.x, pos.y)
    }
}

fn generate_picker_square(pixel: Rgba<u8>) -> impl Iterator<Item = Rgba<u8>> {
    const SIZE: usize = 100;
    const BORDER_THICKNESS: usize = 2;

    let inverted = {
        let [r, g, b, a] = pixel.0;

        Rgba([255 - r, 255 - g, 255 - b, a])
    };

    let horizintal_border = (0..SIZE).map(move |_| inverted);

    let horizintal_inner = (0..(SIZE - (BORDER_THICKNESS * 2))).flat_map(move |_| {
        iter::once(inverted)
            .chain(iter::once(inverted))
            .chain((0..(SIZE - (BORDER_THICKNESS * 2))).map(move |_| pixel))
            .chain(iter::once(inverted))
            .chain(iter::once(inverted))
    });

    horizintal_border
        .clone()
        .chain(horizintal_border.clone())
        .chain(horizintal_inner)
        .chain(horizintal_border.clone())
        .chain(horizintal_border)
}
