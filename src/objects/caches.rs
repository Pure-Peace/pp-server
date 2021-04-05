use std::sync::atomic::{AtomicI32, Ordering};

use actix_web::web::Data;
use async_std::sync::RwLock;
use chrono::{DateTime, Local};
use hashbrown::HashMap;
use peace_performance::Beatmap as PPbeatmap;

use crate::objects::BeatmapCache;
use crate::settings::model::Settings;

use super::Beatmap;

#[derive(Clone)]
pub struct PPbeatmapCache {
    pub beatmap: Data<PPbeatmap>,
    pub time: DateTime<Local>,
}

impl PPbeatmapCache {
    #[inline(always)]
    pub fn new(beatmap: PPbeatmap) -> Self {
        Self {
            beatmap: Data::new(beatmap),
            time: Local::now(),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Data<PPbeatmap> {
        self.beatmap.clone()
    }
}

pub struct BeatmapCaches {
    pub md5: RwLock<HashMap<String, Data<BeatmapCache>>>,
    pub bid: RwLock<HashMap<i32, Data<BeatmapCache>>>,
    pub length: AtomicI32,
}

impl BeatmapCaches {
    pub fn new() -> Self {
        Self {
            md5: RwLock::new(HashMap::with_capacity(200)),
            bid: RwLock::new(HashMap::with_capacity(200)),
            length: AtomicI32::new(0),
        }
    }

    #[inline(always)]
    pub async fn cache(
        &self,
        md5: Option<String>,
        bid: Option<i32>,
        beatmap_cache: Data<BeatmapCache>,
    ) {
        if md5.is_none() && bid.is_none() {
            return;
        };
        if let Some(md5) = md5 {
            self.cache_md5(md5, beatmap_cache.clone()).await;
        };
        if let Some(bid) = bid {
            self.cache_bid(bid, beatmap_cache).await;
        };
        self.length.fetch_add(1, Ordering::SeqCst);
    }

    #[inline(always)]
    pub async fn cache_md5(&self, md5: String, beatmap_cache: Data<BeatmapCache>) {
        let mut md5_w = self.md5.write().await;
        md5_w.insert(md5, beatmap_cache);
    }

    #[inline(always)]
    pub async fn cache_bid(&self, bid: i32, beatmap_cache: Data<BeatmapCache>) {
        let mut bid_w = self.bid.write().await;
        bid_w.insert(bid, beatmap_cache);
    }
}

pub struct Caches {
    pub pp_beatmap_cache: RwLock<HashMap<String, PPbeatmapCache>>,
    pub beatmap_cache: BeatmapCaches,
    pub config: Settings,
}

impl Caches {
    pub fn new(config: Settings) -> Self {
        Self {
            pp_beatmap_cache: RwLock::new(HashMap::with_capacity(200)),
            beatmap_cache: BeatmapCaches::new(),
            config,
        }
    }

    #[inline(always)]
    pub async fn cache_pp_beatmap(&self, md5: String, pp_beatmap_cache: PPbeatmapCache) {
        let mut cw = self.pp_beatmap_cache.write().await;
        if cw.len() as i32 > self.config.beatmap_cache_max {
            debug!("[pp_beatmap_cache] Cache exceed max limit.");
            return;
        };
        cw.insert(md5, pp_beatmap_cache);
    }

    #[inline(always)]
    pub async fn cache_beatmap(
        &self,
        md5: Option<&String>,
        bid: Option<i32>,
        beatmap: Option<&Beatmap>,
    ) {
        if self.beatmap_cache.length.load(Ordering::SeqCst) > self.config.beatmap_cache_max {
            debug!("[beatmap_cache] Cache exceed max limit.");
            return;
        };
        self.beatmap_cache
            .cache(
                md5.cloned(),
                bid,
                Data::new(BeatmapCache::new(beatmap.cloned())),
            )
            .await;
    }

    #[inline(always)]
    pub async fn get_beatmap(
        &self,
        md5: Option<&String>,
        bid: Option<i32>,
    ) -> Option<Data<BeatmapCache>> {
        if let Some(md5) = md5 {
            if let Some(c) = self.beatmap_cache.md5.read().await.get(md5).cloned() {
                return Some(c);
            }
        };

        if let Some(bid) = bid {
            if let Some(c) = self.beatmap_cache.bid.read().await.get(&bid).cloned() {
                return Some(c);
            }
        };

        None
    }
}
