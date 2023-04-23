use image::{RgbImage, DynamicImage};
use winit::monitor::MonitorHandle;
use std::{ptr, slice};
use x11::xlib;
use color_eyre::{Result, eyre::eyre};

/// This will return in the same order as the given `monitors`
pub fn screenshots_ordered(monitors: &[MonitorHandle]) -> Result<Vec<DynamicImage>> {
    let screen = Screen::open().ok_or(eyre!("Could not create screen!"))?;

    monitors
        .iter()
        .map(|monitor| (monitor.position(), monitor.size()))
        .map(|(pos, size)| {

            let image = screen.capture_area(size.width, size.height, pos.x, pos.y).ok_or(eyre!("Could not capture area!"))?;

            Ok(DynamicImage::ImageRgb8(image))
        })
        .collect::<Result<Vec<_>>>()
}

/// A handle to an X11 screen.
pub struct Screen {
    display: *mut xlib::Display,
    window: xlib::Window,
}
#[derive(Debug)]
struct Bgr {
    b: u8,
    g: u8,
    r: u8,
    _pad: u8,
}

impl Screen {
    /// Tries to open the X11 display, then returns a handle to the default screen.
    ///
    /// Returns `None` if the display could not be opened.
    pub fn open() -> Option<Self> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen = xlib::XDefaultScreenOfDisplay(display);
            let root = xlib::XRootWindowOfScreen(screen);
            Some(Self {
                display,
                window: root,
            })
        }
    }

    /// Tries to capture a screenshot of the provided area.
    ///
    /// Returns an `RgbImage` on success, `None` on failure.
    ///
    /// See the documentation of the `image` crate on how to use `RgbImage`.
    pub fn capture_area(&self, w: u32, h: u32, x: i32, y: i32) -> Option<RgbImage> {
        let img =
            unsafe { xlib::XGetImage(self.display, self.window, x, y, w, h, !1, xlib::ZPixmap) };

        if !img.is_null() {
            let image = unsafe { &mut *img };
            let sl: &[Bgr] = unsafe {
                slice::from_raw_parts(
                    (image).data as *const _,
                    (image).width as usize * (image).height as usize,
                )
            };

            let mut bgr_iter = sl.iter();
            let mut image_buffer = RgbImage::new(w, h);

            for pix in image_buffer.pixels_mut() {
                let bgr = bgr_iter.next().unwrap();
                pix.0 = [bgr.r, bgr.g, bgr.b];
            }

            unsafe {
                xlib::XDestroyImage(img as *mut _);
            }
            Some(image_buffer)
        } else {
            None
        }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}