use crate::caches::{BeatmapCache, Caches};
use actix_web::web::Data;
use async_std::fs::File;
use peace_performance::{AnyPP, Beatmap, FruitsPP, ManiaPP, OsuPP, PpResult, TaikoPP};
use serde::Deserialize;

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
pub async fn calculate_pp(beatmap: &Beatmap, data: CalcData) -> PpResult {
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
pub fn mode_calculator(mode: u8, beatmap: &Beatmap) -> AnyPP {
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
) -> Result<Data<Beatmap>, GetBeatmapError> {
    // Try get from beatmap cache
    if let Some(c) = caches.beatmap_cache.read().await.get(md5) {
        let b = c.get();
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
            let c = BeatmapCache::new(b);
            let b = c.get();
            caches.cache_beatmap(md5.to_string(), c).await;
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
