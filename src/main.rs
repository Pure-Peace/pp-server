#![allow(dead_code)]
extern crate config;

#[macro_use]
extern crate log;

pub mod constants;
#[cfg(feature = "peace")]
pub mod database;
#[cfg(feature = "peace")]
use database::Database;

pub mod objects;
pub mod renders;
pub mod routes;
pub mod settings;
pub mod utils;

use actix_web::web::Data;
use objects::{glob::Glob, PPserver};
use settings::model::LocalConfig;

#[actix_web::main]
async fn main() {
    // Create local settings
    let local_config = LocalConfig::init();

    #[cfg(feature = "peace")]
    // Create database object includes postgres and redis pool
    let database = Database::new(&local_config).await;

    // Create Glob object
    let glob = Data::new(
        Glob::init(
            &local_config,
            #[cfg(feature = "peace")]
            &database,
        )
        .await,
    );

    // Create and start
    let mut server = PPserver::new(glob.clone());

    let _err = server.start().await;
}
