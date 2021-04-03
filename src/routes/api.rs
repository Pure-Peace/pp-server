use crate::utils;
use crate::{caches::Caches, settings::model::Settings};
use actix_web::HttpResponse;
use actix_web::{get, web::Data};
use actix_web::{web::Query, HttpRequest};
use serde_json::json;
use std::time::Instant;

use crate::objects::calculator::{self, CalcData};

/// GET "/"
#[get("")]
pub async fn index() -> HttpResponse {
    let contents = r#"<!DOCTYPE html>
    <html lang="en">
      <head>
        <meta charset="utf-8">
        <title>Api</title>
      </head>
      <body>
        <h1>Hello~ PP-server is DEVELOPING.</h1>
        <p>Please access <a href="/api/calc">/api/calc</a> to calculate pp, usage:</p>
        <p><a href="/api/calc?md5=ccb1f31b5eeaf26d40f8c905293efc03">/api/calc?md5=ccb1f31b5eeaf26d40f8c905293efc03</a></p>
        <p>Optional:</p>
        <p>mode (0 = osu!, 1 = Taiko, 2 = CtB, 3 = osu!mania).</p>
        <p>mods (<a href="https://github.com/ppy/osu-api/wiki">See osu-api/wiki</a>)</p>
        <p>n50 (Count of 50)</p>
        <p>n100 (Count of 100)</p>
        <p>n300 (Count of 300)</p>
        <p>acc (float: 0-100)</p>
        <p>miss (Count of miss)</p>
        <p>combo (Your combo)</p>
        <p>katu (Count of katu)</p>
        <p>passed_obj (If failed, use count of passed objects)</p>
      </body>
    </html>"#;
    HttpResponse::Ok().body(contents)
}

// calculate pp (used by peace)
#[get("/calc")]
pub async fn calculate_pp(
    req: HttpRequest,
    config: Data<Settings>,
    caches: Data<Caches>,
) -> HttpResponse {
    let failed = |status, message| {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(json!(
                {
                    "status": status,
                    "message": message,
                    "pp": null
                }
            ))
    };
    let start = Instant::now();
    let data = match Query::<CalcData>::from_query(&req.query_string()) {
        Ok(Query(q)) => q,
        Err(err) => {
            return failed(0, err.to_string().as_str());
        }
    };

    if data.md5.is_none() {
        return failed(0, "md5 not exists");
    };
    // Safe input string
    let md5 = utils::safe_string(data.md5.clone().unwrap());
    // Check md5
    if md5.len() != 32 {
        return failed(0, "invalid md5");
    }

    let beatmap =
        match calculator::get_beatmap_from_local(&md5, &config.osu_files_dir, &caches).await {
            Ok(b) => b,
            Err(err) => {
                return failed(err.error_status(), err.error_message());
            }
        };

    let result = calculator::calculate_pp(&beatmap, data).await;
    let end = start.elapsed();
    info!(
        "[calculate_pp] Beatmap '{}' calculate done in: {:?}",
        md5, end
    );

    HttpResponse::Ok()
        .content_type("application/json")
        .body(json!({
            "status": 1,
            "message": "done",
            "pp": result.pp()
        }))
}
