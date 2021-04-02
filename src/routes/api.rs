use crate::{caches::Caches, settings::model::Settings};
use actix_web::HttpResponse;
use actix_web::{get, web::Data};
use actix_web::{web::Query, HttpRequest};
use async_std::fs::File;
use peace_performance::{AnyPP, Beatmap, FruitsPP, ManiaPP, OsuPP, TaikoPP};
use serde::Deserialize;
use serde_json::json;
use std::{time::Instant, usize};

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
        <h1>Api</h1>
        <p>Please access /calc to calculate pp, usage:</p>
        <p>/api/calc?md5=beatmap md5</p>
        <p>Optional:</p>
        <p>mods=</p>
        <p>n50=</p>
        <p>n100=</p>
        <p>n300=</p>
        <p>acc=</p>
        <p>miss=</p>
        <p>combo=</p>
        <p>katu=</p>
        <p>passed_obj=</p>
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
    #[derive(Deserialize)]
    pub struct CalcQuery {
        md5: Option<String>,
        mode: Option<u8>,
        mods: Option<u32>,
        n50: Option<usize>,
        n100: Option<usize>,
        n300: Option<usize>,
        katu: Option<usize>,
        acc: Option<f32>,
        passed_obj: Option<usize>,
        combo: Option<usize>,
        miss: Option<usize>,
    }
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
    let data = match Query::<CalcQuery>::from_query(&req.query_string()) {
        Ok(Query(q)) => q,
        Err(err) => {
            return failed(0, err.to_string().as_str());
        }
    };

    if data.md5.is_none() {
        return failed(0, "md5 not exists");
    };
    let md5 = data.md5.unwrap();

    let beatmap = match get_beatmap(&md5, &config.osu_files_dir, &caches).await {
        Ok(b) => b,
        Err(err) => {
            return failed(err.error_status(), err.error_message());
        }
    };

    // Get target mode calculator
    let c = mode_calculator(data.mode.unwrap_or(5), &beatmap);
    let c = match data.mods {
        Some(mods) => c.mods(mods),
        None => c,
    };
    let c = match data.combo {
        Some(combo) => c.combo(combo),
        None => c,
    };
    let c = match data.n50 {
        Some(n50) => c.n50(n50),
        None => c,
    };
    let c = match data.n100 {
        Some(n100) => c.n100(n100),
        None => c,
    };
    let c = match data.n300 {
        Some(n300) => c.n300(n300),
        None => c,
    };
    let c = match data.katu {
        Some(katu) => c.n_katu(katu),
        None => c,
    };
    let c = match data.miss {
        Some(miss) => c.misses(miss),
        None => c,
    };
    let c = match data.acc {
        Some(acc) => c.accuracy(acc),
        None => c,
    };
    let c = match data.passed_obj {
        Some(passed_obj) => c.passed_objects(passed_obj),
        None => c,
    };

    // Calculate pp
    let result = c.calculate().await;
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

#[inline(always)]
pub fn mode_calculator(mode: u8, beatmap: &Beatmap) -> AnyPP {
    match mode {
        0 => AnyPP::Osu(OsuPP::new(beatmap)),
        1 => AnyPP::Taiko(TaikoPP::new(beatmap)),
        2 => AnyPP::Fruits(FruitsPP::new(beatmap)),
        3 => AnyPP::Mania(ManiaPP::new(beatmap)),
        _ => AnyPP::new(beatmap),
    }
}

pub enum GetBeatmapError {
    FileNotFound,
    ParseError,
}

impl GetBeatmapError {
    #[inline(always)]
    pub fn error_message(&self) -> &'static str {
        match self {
            Self::FileNotFound => "cannot find .osu file",
            Self::ParseError => "cannot parse .osu file",
        }
    }

    #[inline(always)]
    pub fn error_status(&self) -> i32 {
        match self {
            Self::FileNotFound => -1,
            Self::ParseError => -2,
        }
    }
}

#[inline(always)]
pub async fn get_beatmap(
    md5: &String,
    dir: &String,
    caches: &Data<Caches>,
) -> Result<Data<Beatmap>, GetBeatmapError> {
    // Try get from beatmap cache
    if let Some(b) = caches.beatmap_cache.read().await.get(md5).cloned() {
        debug!("[calculate_pp] Get beatmap '{}' from cache.", md5);
        return Ok(b);
    };

    // Try read .osu file
    let file = match File::open(format!("{}/{}.osu", dir, md5)).await {
        Ok(file) => file,
        Err(err) => {
            error!(
                "[calculate_pp] Cannot find .osu file, md5: '{}', err: {:?}",
                md5, err
            );
            return Err(GetBeatmapError::FileNotFound);
        }
    };

    // Try parse .osu file
    match Beatmap::parse(file).await {
        Ok(b) => {
            let b = Data::new(b.clone());
            caches
                .beatmap_cache
                .write()
                .await
                .insert(md5.to_string(), b.clone());
            Ok(b)
        }
        Err(err) => {
            error!(
                "[calculate_pp] Cannot parse beatmap file, md5: '{}', err: {:?}",
                md5, err
            );
            return Err(GetBeatmapError::ParseError);
        }
    }
}
