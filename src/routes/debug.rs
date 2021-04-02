use actix_web::{dev::Server, get, web::Data, HttpResponse};
use async_std::channel::Sender;
use std::time::Instant;

/// GET "/"
#[get("/")]
pub async fn index() -> HttpResponse {
  let contents = r#"<!DOCTYPE html>
    <html lang="en">
      <head>
        <meta charset="utf-8">
        <title>Hello!</title>
      </head>
      <body>
        <h1>Hello!</h1>
        <p>Hi from Rust</p>
      </body>
    </html>"#;
  HttpResponse::Ok().body(contents)
}

/// GET "/server_stop"
#[get("/server_stop")]
pub async fn server_stop(sender: Data<Sender<Option<Server>>>) -> HttpResponse {
  let start = Instant::now();
  let _ = sender.send(None).await;
  let end = start.elapsed();
  HttpResponse::Ok().body(format!("done in: {:?}", end))
}
