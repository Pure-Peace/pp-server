use crate::{database::Database, utils};
use chrono::{DateTime, Utc};
use tokio_pg_mapper_derive::PostgresMapper;

#[pg_mapper(table = "bancho.config")]
#[derive(Clone, Debug, PostgresMapper)]
/// Bancho config
pub struct BanchoConfig {
    pub name: String,
    pub update_time: DateTime<Utc>,
    pub osu_api_keys: Vec<String>,
    pub timeout_beatmap_cache: i64,
}

impl BanchoConfig {
    #[inline(always)]
    /// Initial bancho config from database
    pub async fn from_database(database: &Database) -> Option<BanchoConfig> {
        utils::struct_from_database(
            "bancho",
            "config",
            "enabled",
            "name, update_time, osu_api_keys, timeout_beatmap_cache",
            &true,
            database,
        )
        .await
    }

    #[inline(always)]
    /// Update bancho config from database
    pub async fn update(&mut self, database: &Database) -> bool {
        let start = std::time::Instant::now();
        let new = BanchoConfig::from_database(database).await;
        if new.is_none() {
            error!("BanchoConfig update failed.");
            return false;
        };
        *self = new.unwrap();
        info!(
            "New BanchoConfig ({}) updated in {:?}; update time: {}",
            self.name,
            start.elapsed(),
            self.update_time
        );
        true
    }
}
