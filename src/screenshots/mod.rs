#[cfg(feature = "screenshots_crate")]
mod screenshots_crate;
#[cfg(feature = "screenshots_crate")]
pub use screenshots_crate::screenshots_ordered;

#[cfg(feature = "flameshot")]
mod flameshot;
#[cfg(feature = "flameshot")]
pub use flameshot::screenshots_ordered;
