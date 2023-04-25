use std::fmt::Display;

use clap::{Parser, ValueEnum};

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
    pub zoom: bool,

    /// Zoom level, this multiplies the zoom size by 2^scale for the preview size. Anything above 10 becomes ridiculous
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(2..))]
    pub scale: u32,

    /// The size of the square captured by the zoom, will crash if not a odd number
    #[arg(long, default_value_t = 11, value_parser = valid_zoom_size)]
    pub size: u32,

    /// The color format that will be printed and put in your clipboard
    #[arg(short, long, default_value_t = ColorFormat::Hex)]
    pub format: ColorFormat,

    /// Disables the terminal colors, useful for when piping to other programs
    #[arg(long, default_value_t = false)]
    pub disable_color: bool,

    /// Disables the clipboard
    #[arg(long, default_value_t = false)]
    pub disable_clipboard: bool,

    /// This potentially helps with with fullscreening issues
    #[arg(long, default_value_t = false)]
    pub exclusive: bool,
}

fn valid_zoom_size(s: &str) -> Result<u32, String> {
    match s.parse() {
        Ok(num) if num % 2 == 1 => Ok(num),
        Ok(num) => Err(format!("{num} is not a odd number!")),
        Err(err) => Err(err.to_string()),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ColorFormat {
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
