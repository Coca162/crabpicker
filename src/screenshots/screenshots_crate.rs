use std::collections::HashMap;

use color_eyre::{eyre::eyre, Result};
use image::{DynamicImage, ImageFormat};
use screenshots::Screen;
use winit::monitor::MonitorHandle;

/// This will return in the same order as the given `monitors`
pub fn screenshots_ordered(monitors: &[MonitorHandle]) -> Result<Vec<DynamicImage>> {
    let mut screens = Screen::all()
        .map_err(|err| eyre!(err))?
        .into_iter()
        .map(|screen| ((screen.display_info.x, screen.display_info.y), screen))
        .collect::<HashMap<_, _>>();

    monitors
        .iter()
        .map(|monitor| monitor.position())
        .map(|pos| {
            let (_, screen) = screens
                .remove_entry(&(pos.x, pos.y))
                .ok_or(eyre!("`screenshots` screens do not match winit monitors!"))?;

            let capture = screen.capture().map_err(|err| eyre!(err))?;

            image::load_from_memory_with_format(capture.buffer(), ImageFormat::Png)
                .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()
}
