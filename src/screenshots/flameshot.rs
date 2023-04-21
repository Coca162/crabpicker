use std::process::Command;

use color_eyre::Result;
use image::{DynamicImage, ImageFormat};
use winit::monitor::MonitorHandle;

/// This will return in the same order as the given `monitors`
pub fn screenshots_ordered(monitors: &[MonitorHandle]) -> Result<Vec<DynamicImage>> {
    let output = Command::new("flameshot")
        .args(["full", "--raw"])
        .output()?
        .stdout;

    let full = image::load_from_memory_with_format(&output, ImageFormat::Png)?;

    Ok(monitors
        .iter()
        .map(|monitor| {
            let pos = monitor.position().cast();
            let size = monitor.size();
            full.crop_imm(pos.x, pos.y, size.width, size.height)
        })
        .collect())
}
