mod api;
mod debug;
mod default;

use actix_web::dev::HttpServiceFactory;
use actix_web::web::{scope, ServiceConfig};

use crate::settings::model::Settings;

/// Function that will be called on new Application to configure routes for this module
/// Initial all routes
pub fn init(cfg: &mut ServiceConfig, settings: &Settings) {
    init_default(cfg);
    cfg.service(init_api());

    // !warning: only debug!
    if settings.debug == true {
        init_debug(cfg)
    }
}

/// Routes for api
fn init_api() -> impl HttpServiceFactory {
    use api::*;
    scope("/api").service(index).service(calculate_pp)
}

fn init_debug(cfg: &mut ServiceConfig) {
    use debug::*;
    cfg.service(index);
    cfg.service(server_stop);
}

/// Routes for default
fn init_default(cfg: &mut ServiceConfig) {
    use default::*;
    cfg.service(index);
}
