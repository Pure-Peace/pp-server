use actix_web::web::Data;
#[cfg(feature = "peace")]
use async_std::sync::RwLock;

use super::{Caches, OsuApi};
use crate::{renders::MainPage, utils};
use crate::settings::model::LocalConfig;

#[cfg(feature = "peace")]
use crate::database::Database;
#[cfg(feature = "peace")]
use crate::settings::bancho::BanchoConfig;

pub struct Glob {
    #[cfg(feature = "peace")]
    pub osu_api: Data<RwLock<OsuApi>>,
    #[cfg(not(feature = "peace"))]
    pub osu_api: Data<OsuApi>,

    pub caches: Data<Caches>,
    pub render_main_page: Data<MainPage>,
    pub local_config: LocalConfig,
    #[cfg(feature = "peace")]
    pub database: Data<Database>,
    #[cfg(feature = "peace")]
    pub config: Data<RwLock<BanchoConfig>>,
}

impl Glob {
    pub async fn init(
        local_config: &LocalConfig,
        #[cfg(feature = "peace")] database: &Database,
    ) -> Self {
        // Create...
        #[cfg(feature = "peace")]
        let config = utils::lock_wrapper(BanchoConfig::from_database(&database).await.unwrap());
        #[cfg(feature = "peace")]
        let osu_api = utils::lock_wrapper(OsuApi::new(&config).await);
        #[cfg(not(feature = "peace"))]
        let osu_api = Data::new(OsuApi::new(&local_config.data.osu_api_keys).await);

        let render_main_page = Data::new(MainPage::new());
        let caches = Data::new(Caches::new(local_config.data.clone()));

        Glob {
            osu_api,
            #[cfg(feature = "peace")]
            database: Data::new(database.clone()),
            caches,
            render_main_page,
            #[cfg(feature = "peace")]
            config,
            local_config: local_config.clone(),
        }
    }
}
