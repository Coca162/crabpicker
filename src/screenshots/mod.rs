#[cfg(feature = "screenshots_crate")]
mod screenshots_crate;
#[cfg(feature = "screenshots_crate")]
pub use screenshots_crate::screenshots_ordered;

#[cfg(feature = "flameshot")]
mod flameshot;
#[cfg(feature = "flameshot")]
pub use flameshot::screenshots_ordered;

#[cfg(feature = "x11")]
mod x11_impl;
#[cfg(feature = "x11")]
pub use x11_impl::screenshots_ordered;
