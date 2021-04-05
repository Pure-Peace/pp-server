mod beatmaps_api;
mod client;
pub mod errors;
mod requester;

pub use beatmaps_api::*;
pub use client::*;
pub use requester::*;

mod depends {
    pub use async_std::sync::RwLock;

    pub use crate::objects::BeatmapFromApi;
    pub use reqwest::Response;
    pub use serde::{de::DeserializeOwned, Serialize};
    pub use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    #[cfg(feature = "peace")]
    pub use crate::settings::bancho::BanchoConfig;
}
