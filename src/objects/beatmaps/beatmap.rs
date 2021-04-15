use super::{depends::*, BeatmapFromApi, GetBeatmapMethod};
use crate::{
    constants::RankStatusInServer,
    objects::{errors::ApiError, glob::Glob},
    utils,
};

#[cfg_attr(
    feature = "peace",
    pg_mapper(table = "beatmaps.maps"),
    derive(FromSql, ToSql, PostgresMapper)
)]
#[derive(Debug, FieldNames, Clone)]
pub struct Beatmap {
    pub server: String,
    pub id: i32,
    pub set_id: i32,
    pub md5: String,
    pub title: String,
    pub artist: String,
    pub diff_name: String,
    pub mapper: String,
    pub mapper_id: i32,
    pub rank_status: i32,
    pub mode: i16,
    pub length: i32,
    pub length_drain: i32,
    pub max_combo: Option<i32>,
    pub fixed_rank_status: bool,
    pub ranked_by: Option<String>,
    pub last_update: Option<DateTime<Local>>,
    pub update_time: DateTime<Local>,
}

impl Beatmap {
    #[inline(always)]
    pub fn get_query_fields() -> String {
        format!("\"{}\"", Beatmap::FIELDS.join("\",\""))
    }

    #[inline(always)]
    pub fn is_unranked(&self) -> bool {
        self.rank_status < 1
    }

    #[inline(always)]
    pub fn is_ranked(&self) -> bool {
        self.rank_status > 0 && self.rank_status != 4
    }

    #[inline(always)]
    pub fn is_qualified(&self) -> bool {
        self.rank_status == 3
    }

    #[inline(always)]
    pub fn rank_status_in_server(&self) -> RankStatusInServer {
        RankStatusInServer::from_api_rank_status(self.rank_status)
    }

    #[inline(always)]
    pub fn file_name(&self) -> String {
        utils::safe_file_name(format!(
            "{artist} - {title} ({mapper}) [{diff_name}].osu",
            artist = self.artist,
            title = self.title,
            mapper = self.mapper,
            diff_name = self.diff_name
        ))
    }

    #[inline(always)]
    /// Get beatmap with MD5 or SID, from local, database, osu!api.
    ///
    /// if `try_from_cache` is true, will try get it from local cache or database first.
    /// if success to get a map from osu!api, will auto cache it to local and database.
    /// if failed to get a map from osu!api, will auto cache it to local as "not submit".
    ///
    /// cache expires seconds can be setted in database (glob.config.timeout_beatmap_cache),
    /// default is 3600s (one hour)
    ///
    ///
    pub async fn get(
        md5: Option<&String>,
        bid: Option<i32>,
        sid: Option<i32>,
        file_name: Option<&String>,
        glob: &Glob,
        try_from_cache: bool,
    ) -> Option<Self> {
        let cache = &glob.caches;

        #[cfg(feature = "peace")]
        let database = &glob.database;

        #[cfg(feature = "peace")]
        let expire = glob.config.read().await.timeout_beatmap_cache;

        #[cfg(not(feature = "peace"))]
        let expire = glob.local_config.data.beatmap_cache_timeout as i64;

        let mut backup_beatmap = None;

        // Try get beatmap from local cache or database
        if try_from_cache {
            // Get from local cache
            if let Some(c) = cache.get_beatmap(md5, bid).await {
                if !c.is_expired(expire) {
                    info!("[Beatmap] Get from cache: {:?}({:?});", md5, bid);
                    return c.beatmap.clone();
                };
                debug!(
                    "[Beatmap] get beatmap {:?}({:?}) from cache but expired, cache time: {:?}",
                    md5, bid, c.create_time
                );
                backup_beatmap = c.beatmap.clone();
            };

            // Local cache expired or not founded, then
            // Try get beatmap from database
            // If get, will auto cache it to local.
            #[cfg(feature = "peace")]
            if let Some(md5) = md5 {
                if let Some(b) = Self::from_database(md5, &GetBeatmapMethod::Md5, database).await {
                    // If not expired, cache it locally and returns it.
                    if !b.is_expired(expire) {
                        cache.cache_beatmap(Some(&md5), bid, Some(&b)).await;
                        return Some(b);
                    }
                    debug!(
                        "[Beatmap] get beatmap {:?}({:?}) from database but expired, cache time: {:?}",
                        md5, bid, b.update_time
                    );
                    backup_beatmap = Some(b);
                };
            };
            #[cfg(feature = "peace")]
            if let Some(bid) = bid {
                if let Some(b) = Self::from_database(&bid, &GetBeatmapMethod::Bid, database).await {
                    // If not expired, cache it locally and returns it.
                    if !b.is_expired(expire) {
                        cache.cache_beatmap(md5, Some(bid), Some(&b)).await;
                        return Some(b);
                    }
                    debug!(
                        "[Beatmap] get beatmap {:?}({:?}) from database but expired, cache time: {:?}",
                        md5, bid, b.update_time
                    );
                    backup_beatmap = Some(b);
                };
            };
        };

        // MD5 Available
        if let Some(md5) = md5 {
            // Cache expired or not founded, then
            // Try get beatmap from osu!api (try with md5)
            // If get, will auto cache it to local and database.
            match Self::from_osu_api(
                md5,
                &GetBeatmapMethod::Md5,
                None,
                &glob.osu_api,
                #[cfg(feature = "peace")]
                database,
            )
            .await
            {
                Ok(b) => {
                    cache.cache_beatmap(Some(&md5), Some(b.id), Some(&b)).await;
                    return Some(b);
                }
                Err(err) => {
                    // If request error, we will not cache it.
                    debug!("[Beatmap] Failed to get beatmap ({}), err: {:?};", md5, err);
                    if err != ApiError::RequestError {
                        // Else, cache it Not submitted
                        cache.cache_beatmap(Some(&md5), bid, None).await;
                    };
                }
            };
        };

        // Cannot get from osu!api by md5, then
        // If SID Available,
        // Try get beatmap from osu!api (try with sid and file name)
        if let Some(bid) = bid {
            match Self::from_osu_api(
                &bid,
                &GetBeatmapMethod::Bid,
                None,
                &glob.osu_api,
                #[cfg(feature = "peace")]
                database,
            )
            .await
            {
                Ok(b) => {
                    cache
                        .cache_beatmap(Some(&b.md5), Some(b.id), Some(&b))
                        .await;
                    return Some(b);
                }
                Err(err) => {
                    // If request error, we will not cache it.
                    debug!("[Beatmap] Failed to get beatmap ({}), err: {:?};", bid, err);
                    if err != ApiError::RequestError {
                        // Else, cache it Not submitted
                        cache.cache_beatmap(md5, Some(bid), None).await;
                    };
                }
            };
        };

        // Cannot get from osu!api by md5 and bid, then
        // If SID Available,
        // Try get beatmap from osu!api (try with sid and file name)
        if let Some(sid) = sid {
            match Self::from_osu_api(
                &sid,
                &GetBeatmapMethod::Sid,
                file_name,
                &glob.osu_api,
                #[cfg(feature = "peace")]
                database,
            )
            .await
            {
                Ok(b) => {
                    let md5 = md5.unwrap_or(&b.md5).clone();
                    cache.cache_beatmap(Some(&md5), Some(b.id), Some(&b)).await;
                    return Some(b);
                }
                Err(err) => {
                    debug!("[Beatmap] Failed to get beatmap ({}), err: {:?};", sid, err);
                    // If request error, we will not cache it as "not submit".
                    if err != ApiError::RequestError {
                        // Else, cache it Not submitted
                        cache.cache_beatmap(md5, bid, None).await;
                    };
                }
            };
        };

        if backup_beatmap.is_none() {
            info!(
                "[Beatmap] Failed to get beatmaps anyway, md5: {:?}, bid: {:?}, sid: {:?}.",
                md5, bid, sid
            );
        } else {
            info!(
                "[Beatmap] Get may outdated beatmap, fail to update beatmap cache. md5: {:?}, bid: {:?}, sid: {:?}.",
                md5, bid, sid
            );
        }
        backup_beatmap
    }

    #[inline(always)]
    // from cache by bid, sid...
    pub async fn from_cache(
        beatmap_md5: &String,
        bid: i32,
        cache: &Caches,
        expire: i64,
    ) -> Result<Option<Self>, ()> {
        debug!("[Beatmap] try get beatmap {} from cache...", beatmap_md5);
        let c = cache.get_beatmap(Some(beatmap_md5), Some(bid)).await;
        if let Some(c) = c {
            // Check is expires
            if c.is_expired(expire) {
                debug!(
                    "[Beatmap] get beatmap {} from cache but expired, cache time: {:?}",
                    beatmap_md5, c.create_time
                );
                return Err(());
            };
            return Ok(c.beatmap.clone());
        };
        // Not in cache
        Err(())
    }

    #[inline(always)]
    pub async fn from_osu_api<T: Any + Display>(
        key: &T,
        method: &GetBeatmapMethod,
        file_name: Option<&String>,
        #[cfg(feature = "peace")] osu_api: &Data<RwLock<OsuApi>>,
        #[cfg(not(feature = "peace"))] osu_api: &Data<OsuApi>,
        #[cfg(feature = "peace")] database: &Database,
    ) -> Result<Self, ApiError> {
        Ok(BeatmapFromApi::from_osu_api(
            key,
            method,
            file_name,
            osu_api,
            #[cfg(feature = "peace")]
            database,
        )
        .await?
        .convert_to_beatmap())
    }

    #[cfg(feature = "peace")]
    #[inline(always)]
    pub async fn from_database<T: Any + Display>(
        key: &T,
        method: &GetBeatmapMethod,
        database: &Database,
    ) -> Option<Self> {
        let v = (key as &dyn Any).downcast_ref::<String>().unwrap();
        debug!(
            "[Beatmap] Try get with Method({:?}) {} from database...",
            method, key
        );
        utils::struct_from_database(
            "beatmaps",
            "maps",
            method.db_column_name().as_str(),
            Self::get_query_fields().as_str(),
            v,
            database,
        )
        .await
    }

    #[cfg(feature = "peace")]
    #[inline(always)]
    pub async fn from_database_by_bid(beatmap_id: i32, database: &Database) -> Option<Self> {
        Self::from_database(&beatmap_id, &GetBeatmapMethod::Bid, database).await
    }

    #[cfg(feature = "peace")]
    #[inline(always)]
    pub async fn from_database_by_sid(beatmap_set_id: i32, database: &Database) -> Option<Self> {
        Self::from_database(&beatmap_set_id, &GetBeatmapMethod::Sid, database).await
    }

    #[cfg(feature = "peace")]
    #[inline(always)]
    pub async fn from_database_by_md5(beatmap_md5: &String, database: &Database) -> Option<Self> {
        Self::from_database(beatmap_md5, &GetBeatmapMethod::Md5, database).await
    }

    #[inline(always)]
    pub fn is_expired(&self, expires: i64) -> bool {
        // Fixed never expire!
        if self.fixed_rank_status {
            return false;
        }
        (Local::now() - self.update_time).num_seconds() > expires
    }
}

impl From<BeatmapFromApi> for Beatmap {
    fn from(f: BeatmapFromApi) -> Self {
        Self {
            server: "ppy".to_string(),
            id: f.id,
            set_id: f.set_id,
            md5: f.md5,
            title: f.title,
            artist: f.artist,
            diff_name: f.diff_name,
            mapper: f.mapper,
            mapper_id: f.mapper_id,
            rank_status: f.rank_status,
            mode: f.mode,
            length: f.length,
            length_drain: f.length_drain,
            max_combo: f.max_combo,
            fixed_rank_status: [1, 2].contains(&f.rank_status),
            ranked_by: None,
            last_update: f.last_update,
            update_time: Local::now(),
        }
    }
}
