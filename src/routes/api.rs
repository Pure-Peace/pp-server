use crate::utils;
use crate::Glob;
use actix_web::HttpResponse;
use actix_web::{get, web::Data};
use actix_web::{web::Query, HttpRequest};
use calculator::GetBeatmapError;
use serde_json::json;
use std::time::Instant;

use crate::objects::calculator::{self, CalcData};
use crate::objects::Beatmap;

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
        <P>Sample</P>
        <p>WITH MD5: <a href="/api/calc?md5=ccb1f31b5eeaf26d40f8c905293efc03">/api/calc?md5=ccb1f31b5eeaf26d40f8c905293efc03</a></p>
        <p>WITH BID: <a href="/api/calc?bid=2848898">/api/calc?bid=2848898</a></p>
        <P>WITH SID + FILENAME: <a href="/api/calc?sid=1378720&file_name=Tanchiky%20-%20Bridge%20(NyarkoO)%20[Extension].osu">/api/calc?sid=1378720&file_name=Tanchiky%20-%20Bridge%20(NyarkoO)%20[Extension].osu</a></P>
        <p>Optional:</p>
        <p>md5 (Get beatmap with md5) *Recommend</p>
        <p>bid (Get beatmap with bid)</p>
        <p>sid (Get beatmap with sid and file_name)</p>
        <p>file_name ({artist} - {title} ({mapper}) [{diff_name}].osu)</p>
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
pub async fn calculate_pp(req: HttpRequest, glob: Data<Glob>) -> HttpResponse {
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

    // Parse query data
    let data = match Query::<CalcData>::from_query(&req.query_string()) {
        Ok(Query(q)) => q,
        Err(err) => {
            return failed(0, err.to_string().as_str());
        }
    };

    // We need any one of these
    if data.md5.is_none() && data.bid.is_none() && data.sid.is_none() {
        return failed(
            0,
            "invalid requests, we must have one of: (md5, bid, sid + filename)",
        );
    };

    let (md5, bid) = if data.md5.is_none() {
        // If have not md5, we should get beatmap first
        let b = match Beatmap::get(
            None,
            data.bid,
            data.sid,
            data.file_name.as_ref(),
            &glob,
            true,
        )
        .await
        {
            Some(b) => b,
            None => return failed(0, "cannot get beatmap from anyway"),
        };
        debug!("[calculate_pp] without md5, but get success: {}({})", b.md5, b.id);
        (b.md5, Some(b.id))
    } else {
        // Safe input string
        (utils::safe_string(data.md5.clone().unwrap()), data.bid)
    };

    // Check md5
    if md5.len() != 32 {
        return failed(0, "invalid md5");
    }

    // Try get beatmap from local or osu!api
    let beatmap = match calculator::get_beatmap_from_local(
        &md5,
        &glob.local_config.data.osu_files_dir,
        &glob.caches,
    )
    .await
    {
        Ok(b) => b,
        Err(err) => {
            // Cannot parse
            if err == GetBeatmapError::ParseError {
                return failed(err.error_status(), err.error_message());
            };
            // Not found
            match calculator::get_beatmap_from_api(&md5, bid, &glob).await {
                Some(b) => b,
                None => return failed(err.error_status(), err.error_message()),
            }
        }
    };

    // Get it, calculate.
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
