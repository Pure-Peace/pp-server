use actix_web::{get, web::Data, HttpResponse};
use askama::Template;

use crate::objects::glob::Glob;

/// GET "/"
#[get("/")]
pub async fn index(glob: Data<Glob>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(glob.render_main_page.render().unwrap())
}
