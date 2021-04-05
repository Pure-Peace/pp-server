use actix_web::{get, web::Data, HttpResponse};
use askama::Template;
use prometheus::IntCounterVec;

use crate::objects::glob::Glob;

/// GET "/"
#[get("/")]
pub async fn index(glob: Data<Glob>, counter: Data<IntCounterVec>) -> HttpResponse {
    counter
        .with_label_values(&["/bancho", "get", "start"])
        .inc();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(glob.render_main_page.render().unwrap())
}
