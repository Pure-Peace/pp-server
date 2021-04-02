use actix_web::web::Data;
use async_std::sync::RwLock;
use hashbrown::HashMap;
use peace_performance::Beatmap;

pub struct Caches {
    pub beatmap_cache: RwLock<HashMap<String, Data<Beatmap>>>,
}

impl Caches {
    pub fn new() -> Self {
        Self {
            beatmap_cache: RwLock::new(HashMap::new()),
        }
    }
}
