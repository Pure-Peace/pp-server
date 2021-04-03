#[cfg(feature = "peace")]
mod beatmaps;
#[cfg(feature = "peace")]
pub use beatmaps::*;
#[cfg(feature = "peace")]
mod osu_api;
#[cfg(feature = "peace")]
pub use osu_api::*;

mod server;
pub use server::PPserver;

pub mod calculator;
