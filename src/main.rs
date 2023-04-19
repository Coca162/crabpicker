#![warn(clippy::all, clippy::nursery)]

use copypasta_ext::{prelude::ClipboardProvider, x11_bin::ClipboardContext};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use winit::event_loop::EventLoop;

pub mod window_pickers;

use window_pickers::get_color;

fn main() {
    let mut event_loop = EventLoop::new();

    loop {
        let (r, g, b) = get_color(&mut event_loop).unwrap();

        let rgb_hex = format!("#{r:02X}{g:02X}{b:02X}");

        print_result((r, g, b), &rgb_hex);

        let clip_res = ClipboardContext::new().and_then(|mut x| x.set_contents(rgb_hex));

        if clip_res.is_err() {
            println!("Failed to set clipboard content (do you have xclip?)");
        }

        println!("bruh!");

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
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
