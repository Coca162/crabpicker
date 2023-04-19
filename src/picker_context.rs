use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use image::{DynamicImage, GenericImageView, ImageFormat};
use screenshots::Screen;
use softbuffer::{GraphicsContext, SoftBufferError};
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};

pub struct PickerContext {
    _windows: Vec<Window>,
    graphics: HashMap<WindowId, (GraphicsContext, DynamicImage)>,
}

impl PickerContext {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let screens = Screen::all().map_err(|err| eyre!(err))?;

        let mut monitors = event_loop
            .available_monitors()
            .map(|monitor| (monitor.position(), monitor))
            .collect::<HashMap<_, _>>();

        let windows = screens
            .iter()
            .map(|screen| {
                let info = screen.display_info;
                let (_, monitor) = monitors
                    .remove_entry(&PhysicalPosition::new(info.x, info.y))
                    .ok_or(eyre!("`screenshots` screens do not match winit monitors!"))?;
                
                WindowBuilder::new()
                    .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))))
                    .build(event_loop)
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()?;

        let images = screens
            .iter()
            .map(|screen| screen.capture().map_err(|err| eyre!(err)))
            .map(|image| {
                image.and_then(|image| {
                    image::load_from_memory_with_format(image.buffer(), ImageFormat::Png)
                        .map_err(Into::into)
                })
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
            _windows: windows,
            graphics,
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
        _position: Option<(PhysicalPosition<f64>, WindowId)>,
    ) -> Option<()> {
        let (graphics_ctx, ref image) = self.graphics.get_mut(window_id)?;

        let buffer: Vec<u32> = image
            .as_rgba8()
            // SAFETY: `screenshots` crate should be returning RGBA8 screenshots
            .unwrap()
            .chunks(4)
            .map(|rgb| rgb[2] as u32 | ((rgb[1] as u32) << 8) | ((rgb[0] as u32) << 16))
            .collect();

        graphics_ctx.set_buffer(&buffer, image.width() as u16, image.height() as u16);

        Some(())
    }
}
