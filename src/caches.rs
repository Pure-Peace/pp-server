use actix_web::web::Data;
use async_std::sync::RwLock;
use chrono::{DateTime, Local};
use hashbrown::HashMap;
use peace_performance::Beatmap;

use crate::settings::model::Settings;

pub struct BeatmapCache {
    pub beatmap: Data<Beatmap>,
    pub time: DateTime<Local>,
}

impl BeatmapCache {
    #[inline(always)]
    pub fn new(beatmap: Beatmap) -> Self {
        Self {
            beatmap: Data::new(beatmap),
            time: Local::now(),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Data<Beatmap> {
        self.beatmap.clone()
    }
}

pub struct Caches {
    pub beatmap_cache: RwLock<HashMap<String, BeatmapCache>>,
    pub config: Settings,
}

impl Caches {
    pub fn new(config: Settings) -> Self {
        Self {
            beatmap_cache: RwLock::new(HashMap::new()),
            config,
        }
    }

    #[inline(always)]
    pub async fn cache_beatmap(&self, md5: String, beatmap_cache: BeatmapCache) {
        let mut beatmap_cache_w = self.beatmap_cache.write().await;
        if beatmap_cache_w.len() as i32 > self.config.beatmap_cache_max {
            debug!("Cache exceed max limit.");
            return;
        };
        beatmap_cache_w.insert(md5, beatmap_cache);
    }
}
