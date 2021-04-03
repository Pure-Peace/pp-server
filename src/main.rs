#![allow(dead_code)]
extern crate config;

#[macro_use]
extern crate log;

pub mod caches;
pub mod constants;
#[cfg(feature = "peace")]
pub mod database;
pub mod objects;
pub mod renders;
pub mod routes;
pub mod settings;
pub mod utils;

use crate::settings::model::LocalConfig;
use objects::PPserver;

#[actix_web::main]
async fn main() {
    // Create local settings
    let local_config = LocalConfig::init();

    // Create and start
    let mut server = PPserver::new(local_config);

    let _err = server.start().await;
}
