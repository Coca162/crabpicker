#![warn(clippy::all, clippy::nursery)]

use clap::{Parser, ValueEnum};
use color_eyre::eyre::Result;
use copypasta_ext::display::DisplayServer;
use image::{Pixel, Rgb};
use std::{fmt::Display, io::Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub mod color_event_loop;
pub mod picker_context;
pub mod screenshots;

use color_event_loop::get_color;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "A GUI color picker with a tooglable zoom via CTRL or on hold with Z
You can either use left click or ENTER to ouput the color to your CLI and your clipboard
When zooming you can use the scroll wheel to change the zoom scale and with shift to change the zoom size
You can move around for precise measurement with wasd, vim or arrow key movement"
)]
pub struct Args {
    /// Turns on zoom at the start
    #[arg(short, long, default_value_t = false)]
    zoom: bool,

    /// Zoom level, this multiplies the zoom_size by 2^scale for the preview size. Anything above 10 becomes ridiculous
    #[arg(short, long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(2..))]
    scale: u32,

    /// The size of the square captured by the zoom, will crash if not a odd number
    #[arg(long, default_value_t = 11, value_parser = valid_zoom_size)]
    zoom_size: u32,

    /// The color format that will be printed and put in your clipboard
    #[arg(long, default_value_t = ColorFormat::Hex)]
    color_format: ColorFormat,

    /// Disables the terminal colors, useful for when piping to other programs
    #[arg(long, default_value_t = false)]
    disable_term_colors: bool,

    /// Disables the clipboard
    #[arg(long, default_value_t = false)]
    disable_clipboard: bool,

    /// This potentially helps with TWMs which are fussy with fullscreening
    #[arg(long, default_value_t = false)]
    exclusive_fullscreen: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ColorFormat {
    /// Saves color in hex code: #RRGGBB
    Hex,
    /// Saves color in rgb: RRR, GGG, BBB
    Rgb,
}

impl Display for ColorFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rgb => write!(f, "rgb"),
            Self::Hex => write!(f, "hex"),
        }
    }
}

fn valid_zoom_size(s: &str) -> Result<u32, String> {
    match s.parse() {
        Ok(num) if num % 2 == 1 => Ok(num),
        Ok(num) => Err(format!("{num} is not a odd number!")),
        Err(err) => Err(err.to_string()),
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    if let Some((r, g, b)) = get_color(&args)? {
        let formatted_rgb = match args.color_format {
            ColorFormat::Hex => format!("#{r:02X}{g:02X}{b:02X}"),
            ColorFormat::Rgb => format!("{r}, {g}, {b}"),
        };

        if !args.disable_term_colors {
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
                println!(
                    "Failed to set clipboard content (do you have xclip/wl-clipboard?): {err}"
                );
            }
        }
    } else {
        println!("Picker was cancelled")
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
