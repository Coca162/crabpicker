#![warn(clippy::all, clippy::nursery)]

use color_eyre::eyre::Result;
use copypasta_ext::display::DisplayServer;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use winit::event_loop::EventLoop;

mod color_event_loop;
mod picker_context;
mod screenshots;

use color_event_loop::get_color;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut event_loop: EventLoop<()> = EventLoop::new();

    if let Some((r, g, b)) = get_color(&mut event_loop)? {
        let rgb_hex = format!("#{r:02X}{g:02X}{b:02X}");

        print_result((r, g, b), &rgb_hex);
        let clip_res = DisplayServer::select()
            .try_context()
            .map(|mut x| x.set_contents(rgb_hex))
            .expect("Could not find display server");

        if let Err(err) = clip_res {
            println!("Failed to set clipboard content (do you have xclip/wl-clipboard?): {err}");
        }
    } else {
        println!("Picker was cancelled")
    }

    Ok(())
}

fn print_result((r, g, b): (u8, u8, u8), rgb_hex: &str) -> Option<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout
        .set_color(
            ColorSpec::new()
                .set_bg(Some(Color::Rgb(r, g, b)))
                .set_fg(Some(Color::Rgb(255 - r, 255 - g, 255 - b))),
        )
        .ok()?;

    stdout.write_all(rgb_hex.as_bytes()).ok()?;

    stdout.reset().ok()?;

    stdout.write_all(&[b'\n']).ok()?;

    Some(())
}
