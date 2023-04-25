#![warn(clippy::all, clippy::nursery)]

use args::{Args, ColorFormat};
use clap::Parser;
use color_eyre::eyre::Result;
use copypasta_ext::display::DisplayServer;
use image::{Pixel, Rgb};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub mod args;
pub mod picker_context;
pub mod picker_event_loop;
pub mod screenshots;

use picker_event_loop::launch_picker_gui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let (r, g, b) = match launch_picker_gui(&args)? {
        Some(rgb) => rgb,
        None => {
            println!("Picker was cancelled");
            return Ok(());
        }
    };

    let formatted_rgb = match args.format {
        ColorFormat::Hex => format!("#{r:02X}{g:02X}{b:02X}"),
        ColorFormat::Rgb => format!("{r}, {g}, {b}"),
    };

    if !args.disable_color {
        print_result((r, g, b), &formatted_rgb);
    } else {
        println!("{formatted_rgb}")
    }

    if !args.disable_clipboard {
        let clip_res = DisplayServer::select()
            .try_context()
            .map(|mut x| x.set_contents(formatted_rgb))
            .expect("Could not find display server");

        if let Err(err) = clip_res {
            println!("Failed to set clipboard content (do you have xclip/wl-clipboard?): {err}");
        }
    }

    Ok(())
}

fn print_result((r, g, b): (u8, u8, u8), rgb_hex: &str) -> Option<()> {
    let background = 255 - Rgb([r, g, b]).to_luma().0[0];

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout
        .set_color(
            ColorSpec::new()
                .set_bg(Some(Color::Rgb(r, g, b)))
                .set_fg(Some(Color::Rgb(background, background, background))),
        )
        .ok()?;

    stdout.write_all(rgb_hex.as_bytes()).ok()?;

    stdout.reset().ok()?;

    stdout.write_all(&[b'\n']).ok()?;

    Some(())
}
