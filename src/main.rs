#![allow(dead_code)]
extern crate config;

#[macro_use]
extern crate log;

pub mod caches;
pub mod constants;
pub mod renders;
pub mod routes;
pub mod server;
pub mod settings;

use crate::settings::model::LocalConfig;
use server::PPserver;

#[actix_web::main]
async fn main() {
    // Create local settings
    let local_config = LocalConfig::init();

    // Create and start
    let mut server = PPserver::new(local_config);

    let _err = server.start().await;
}
