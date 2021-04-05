use crate::objects::caches::{Caches, PPbeatmapCache};
use crate::objects::Beatmap;
use crate::Glob;
use actix_web::web::Data;
use async_std::fs::File;
use bytes::Bytes;
use peace_performance::{AnyPP, Beatmap as PPbeatmap, FruitsPP, ManiaPP, OsuPP, PpResult, TaikoPP};
use serde::Deserialize;
use std::{cmp::PartialEq, time::Instant};

#[derive(PartialEq)]
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

#[derive(Deserialize)]
pub struct CalcData {
    pub md5: Option<String>,
    pub bid: Option<i32>,
    pub sid: Option<i32>,
    pub file_name: Option<String>,
    pub mode: Option<u8>,
    pub mods: Option<u32>,
    pub n50: Option<usize>,
    pub n100: Option<usize>,
    pub n300: Option<usize>,
    pub katu: Option<usize>,
    pub acc: Option<f32>,
    pub passed_obj: Option<usize>,
    pub combo: Option<usize>,
    pub miss: Option<usize>,
}

#[inline(always)]
pub async fn calculate_pp(beatmap: &PPbeatmap, data: CalcData) -> PpResult {
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
    c.calculate().await
}

#[inline(always)]
pub fn mode_calculator(mode: u8, beatmap: &PPbeatmap) -> AnyPP {
    match mode {
        0 => AnyPP::Osu(OsuPP::new(beatmap)),
        1 => AnyPP::Taiko(TaikoPP::new(beatmap)),
        2 => AnyPP::Fruits(FruitsPP::new(beatmap)),
        3 => AnyPP::Mania(ManiaPP::new(beatmap)),
        _ => AnyPP::new(beatmap),
    }
}

#[inline(always)]
pub async fn get_beatmap_from_local(
    md5: &String,
    dir: &String,
    caches: &Data<Caches>,
) -> Result<Data<PPbeatmap>, GetBeatmapError> {
    // Try get from beatmap cache
    if let Some(c) = caches.pp_beatmap_cache.read().await.get(md5) {
        let b = c.get();
        debug!("[calculate_pp] Get beatmap '{}' from cache.", md5);
        return Ok(b);
    };

    // Try read .osu file
    let file = match File::open(format!("{}/{}.osu", dir, md5)).await {
        Ok(file) => file,
        Err(_) => {
            info!("[calculate_pp] Cannot find .osu file, md5: '{}'", md5);
            return Err(GetBeatmapError::FileNotFound);
        }
    };

    // Try parse .osu file
    match PPbeatmap::parse(file).await {
        Ok(b) => {
            let c = PPbeatmapCache::new(b);
            let b = c.get();
            caches.cache_pp_beatmap(md5.to_string(), c).await;
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

#[inline(always)]
pub async fn write_osu_file(bytes: Bytes, path: String) -> bool {
    match async_std::fs::write(path, bytes).await {
        Ok(_) => true,
        Err(err) => {
            warn!(
                "[calculate_pp] Failed to write into .osu file locally, err: {:?}",
                err
            );
            return false;
        }
    }
}

#[inline(always)]
pub async fn get_beatmap_from_api(
    request_md5: &String,
    bid: Option<i32>,
    glob: &Glob,
) -> Option<Data<PPbeatmap>> {
    let start = Instant::now();
    let bid = bid.unwrap_or(
        Beatmap::get(Some(request_md5), None, None, None, &glob, true)
            .await?
            .id,
    );
    // Download beatmap, and try parse it
    let (b, new_md5, bytes) = match glob.osu_api.read().await.get_pp_beatmap(bid).await {
        Ok((b, new_md5, bytes)) => (b, new_md5, bytes),
        Err(err) => {
            warn!(
                "[calculate_pp] Cannot get .osu file from osu!api, err: {:?}",
                err
            );
            return None;
        }
    };

    // Save .osu file locally
    write_osu_file(
        bytes,
        format!("{}/{}.osu", glob.local_config.data.osu_files_dir, new_md5),
    )
    .await;

    // Cache it
    let c = PPbeatmapCache::new(b);
    let b = c.get();
    glob.caches.cache_pp_beatmap(new_md5.clone(), c).await;

    // Check .osu file is same md5
    if &new_md5 != request_md5 {
        warn!("[calculate_pp] Success get .osu file from api, but md5 not eq.");
        return None;
    }

    info!(
        "[calculate_pp] Success get .osu file from api, bid: {}, md5: {}; time spent: {:?}",
        bid,
        request_md5,
        start.elapsed()
    );

    Some(b)
}
