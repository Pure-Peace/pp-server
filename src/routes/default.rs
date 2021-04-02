use crate::renders::MainPage;
use actix_web::{get, web::Data, HttpResponse};
use askama::Template;
use prometheus::IntCounterVec;

/// GET "/"
#[get("/")]
pub async fn index(render: Data<MainPage>, counter: Data<IntCounterVec>) -> HttpResponse {
  counter
    .with_label_values(&["/bancho", "get", "start"])
    .inc();

  HttpResponse::Ok()
    .content_type("text/html")
    .body(render.render().unwrap())
}
