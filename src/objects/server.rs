use crate::objects::calculator::{self, CalcData};
use crate::Glob;
use actix_cors::Cors;
use actix_web::web::Query;
use actix_web::{dev::Server, middleware::Logger, web::Data, App, HttpServer};
use async_std::channel::{unbounded, Receiver, Sender};
use chrono::Local;
use std::time::{Duration, Instant};

use colored::Colorize;

use actix_web_prom::PrometheusMetrics;
use prometheus::{opts, IntCounterVec};

use crate::settings::model::Settings;
use crate::{routes, utils};

pub struct PPserver {
    pub addr: String,
    pub glob: Data<Glob>,
    pub prometheus: PrometheusMetrics,
    pub counter: IntCounterVec,
    pub server: Option<Server>,
    pub sender: Sender<Option<Server>>,
    pub receiver: Receiver<Option<Server>>,
    pub start_time: Option<Instant>,
}

impl PPserver {
    pub fn new(glob: Data<Glob>) -> Self {
        let sets = &glob.local_config.data;
        let addr = glob
            .local_config
            .cfg
            .get("server.addr")
            .unwrap_or("127.0.0.1:8088".to_string());

        // Prometheus
        let (prometheus, counter) = Self::prom_init(&addr, sets);
        let (sender, receiver) = unbounded();

        Self {
            addr,
            glob,
            prometheus,
            counter,
            server: None,
            sender,
            receiver,
            start_time: None,
        }
    }

    pub async fn run_server(&mut self) {
        // Run server
        info!("{}", "Starting http server...".bold().bright_blue());
        let server = {
            let glob_cloned = self.glob.clone();
            let settings_cloned = self.glob.local_config.data.clone();
            let counter = self.counter.clone();
            let sender = Data::new(self.sender.clone());
            let prom = self.prometheus.clone();
            HttpServer::new(move || {
                // App
                App::new()
                    .wrap(Self::make_logger(&settings_cloned))
                    .wrap(prom.clone())
                    .wrap(
                        Cors::default()
                            .allow_any_origin()
                            .allow_any_header()
                            .allow_any_method()
                            .supports_credentials(),
                    )
                    .app_data(sender.clone())
                    .app_data(glob_cloned.clone())
                    .data(counter.clone())
                    .configure(|service_cfg| routes::init(service_cfg, &settings_cloned))
            })
            .shutdown_timeout(2)
            .keep_alive(120)
            .bind(&self.addr)
            .unwrap()
            .run()
        };
        let _ = self.sender.send(Some(server)).await;
        self.start_time = Some(self.started());
    }

    pub async fn start(&mut self) -> std::io::Result<()> {
        let config = &self.glob.local_config.data;
        // Should preload or not
        if config.preload_osu_files {
            utils::preload_osu_files(config, &self.glob.caches).await;
        };

        self.start_auto_cache_clean(config.auto_clean_interval, config.beatmap_cache_timeout)
            .await;
        #[cfg(feature = "peace")]
        self.start_auto_pp_recalculate(
            config.auto_pp_recalculate.interval,
            config.auto_pp_recalculate.max_retry,
        )
        .await;

        self.run_server().await;
        // Wait for stopped
        self.stopped().await
    }

    #[inline(always)]
    // Auto cache clean
    pub async fn start_auto_cache_clean(&self, interval: u64, timeout: u64) {
        let caches = self.glob.caches.clone();
        let duration = Duration::from_secs(interval);
        async_std::task::spawn(async move {
            loop {
                async_std::task::sleep(duration).await;
                debug!("[auto_cache_clean] task started...");
                let start = Instant::now();
                let mut ready_to_clean = Vec::new();
                let now = Local::now().timestamp();

                // Collect cache if timeout
                let pp_beatmap_cache = caches.pp_beatmap_cache.read().await;
                for (k, v) in pp_beatmap_cache.iter() {
                    if now - v.time.timestamp() > timeout as i64 {
                        ready_to_clean.push(k.clone());
                    }
                }
                // release read lock
                drop(pp_beatmap_cache);

                // Clean timeout cache
                if ready_to_clean.len() > 0 {
                    debug!("[auto_cache_clean] Timeout cache founded, will clean them...");
                    let mut pp_beatmap_cache = caches.pp_beatmap_cache.write().await;
                    for k in ready_to_clean {
                        pp_beatmap_cache.remove(&k);
                    }
                }
                debug!(
                    "[auto_cache_clean] task done, time spent: {:?}",
                    start.elapsed()
                );
            }
        });
    }

    #[cfg(feature = "peace")]
    #[inline(always)]
    /// Auto pp recalculate (When pp calculation fails, join the queue and try to recalculate)
    pub async fn start_auto_pp_recalculate(&self, interval: u64, max_retry: i32) {
        let duration = Duration::from_secs(interval);
        let database = self.glob.database.clone();
        let glob = self.glob.clone();
        tokio::task::spawn(async move {
            loop {
                debug!("[auto_pp_recalculate] task started...");
                let mut process: i32 = 0;
                let mut success: i32 = 0;
                let mut failed: i32 = 0;
                let start = Instant::now();
                // Try get tasks from redis
                let keys: Option<Vec<String>> = match database.redis.query("KEYS", "calc:*").await {
                    Ok(k) => Some(k),
                    Err(err) => {
                        error!(
                            "[auto_pp_recalculate] Failed to get redis keys, err: {:?}",
                            err
                        );
                        None
                    }
                };
                if let Some(keys) = keys {
                    let total = keys.len();
                    if total > 0 {
                        debug!(
                            "[auto_pp_recalculate] {} task founded! start recalculate!",
                            keys.len()
                        );
                        for key in keys {
                            process += 1;
                            debug!("[auto_pp_recalculate] task{}/{}: {}", process, total, key);

                            let k = key.split(":").collect::<Vec<&str>>();
                            if k.len() != 4 {
                                warn!(
                                    "[auto_pp_recalculate] Invalid key(key length): {}, remove it;",
                                    key
                                );
                                failed += 1;
                                let _ = database.redis.del(key).await;
                                continue;
                            };

                            // Get key info
                            let table = k[1];
                            let score_id = match k[2].parse::<i64>() {
                                Ok(i) => i,
                                Err(err) => {
                                    warn!("[auto_pp_recalculate] Invalid key(score_id): {}, remove it; err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database.redis.del(key).await;
                                    continue;
                                }
                            };
                            let _player_id = match k[3].parse::<i32>() {
                                Ok(i) => i,
                                Err(err) => {
                                    warn!("[auto_pp_recalculate] Invalid key(player_id): {}, remove it; err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database.redis.del(key).await;
                                    continue;
                                }
                            };

                            // Get key data
                            let s = match database.redis.get::<String, _>(&key).await {
                                Ok(s) => s,
                                Err(err) => {
                                    warn!("[auto_pp_recalculate] Invalid key(data): {}, remove it; err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database.redis.del(key).await;
                                    continue;
                                }
                            };
                            let values = s.split(":").collect::<Vec<&str>>();
                            if values.len() != 2 {
                                warn!("[auto_pp_recalculate] Invalid key(values length): {}, remove it", key);
                                failed += 1;
                                let _ = database.redis.del(key).await;
                                continue;
                            }
                            let try_count = match values[0].parse::<i32>() {
                                Ok(i) => i,
                                Err(err) => {
                                    warn!("[auto_pp_recalculate] Invalid key(try_count): {}, remove it; err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database.redis.del(key).await;
                                    continue;
                                }
                            };
                            if try_count >= max_retry {
                                warn!(
                                    "[auto_pp_recalculate] key {} over max_retry, remove it;",
                                    key
                                );
                                failed += 1;
                                let _ = database.redis.del(key).await;
                                continue;
                            };
                            let data = match Query::<CalcData>::from_query(values[1]) {
                                Ok(Query(d)) => d,
                                Err(err) => {
                                    warn!("[auto_pp_recalculate] Invalid key(calc data parse): {}, remove it; err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database.redis.del(key).await;
                                    continue;
                                }
                            };

                            // get beatmap
                            let beatmap = match calculator::get_beatmap(
                                data.md5.clone(),
                                data.bid,
                                data.sid,
                                data.file_name.clone(),
                                &glob,
                            )
                            .await
                            {
                                Some(b) => b,
                                None => {
                                    warn!("[auto_pp_recalculate] Failed to get beatmap, key: {}, data: {:?}; try_count: {}", key, data, try_count);
                                    failed += 1;
                                    let _ = database
                                        .redis
                                        .set(&key, format!("{}:{}", try_count + 1, values[1]))
                                        .await;
                                    continue;
                                }
                            };
                            // calculate.
                            let r = calculator::calculate_pp(&beatmap, &data).await;

                            // Save it
                            match database.pg.execute(
                                &format!("UPDATE \"game_scores\".\"{}\" SET pp_v2 = $1, pp_v2_raw = $2, stars = $3 WHERE \"id\" = $4", table), &[
                                &r.pp(), &serde_json::json!({
                                    "aim": r.raw.aim,
                                    "spd": r.raw.spd,
                                    "str": r.raw.str,
                                    "acc": r.raw.acc,
                                    "total": r.raw.total,
                                }), &r.stars(), &score_id
                            ]).await {
                                Ok(_) => {
                                    debug!("[auto_pp_recalculate] key {} calculate done", key);
                                    success += 1;
                                    let _ = database.redis.del(key).await;
                                    // TODO: Tell peace to update the status of this player
                                    continue;
                                },
                                Err(err) => {
                                    error!("[auto_pp_recalculate] Failed to save calculate result, key: {}, err: {:?}", key, err);
                                    failed += 1;
                                    let _ = database
                                        .redis
                                        .set(&key, format!("{}:{}", try_count + 1, values[1]))
                                        .await;
                                    continue;
                                }
                            };
                        }
                        info!(
                            "[auto_pp_recalculate] task done, time spent: {:?}; success({}) / total({}) failed({})",
                            start.elapsed(), success, process, failed
                        );
                    }
                };
                tokio::time::delay_for(duration).await;
            }
        });
    }

    /// Server started
    pub fn started(&self) -> Instant {
        // Server started
        let text = format!("Server is Running at http://{}", self.addr)
            .bold()
            .green();
        info!("{}", text);
        Instant::now()
    }

    /// Server stopped
    pub async fn stopped(&self) -> std::io::Result<()> {
        let server = self.receiver.recv().await.unwrap().unwrap();
        // Waiting for server stopped
        let rx = self.receiver.clone();
        let srv = server.clone();
        async_std::task::spawn(async move {
            if let Ok(_) = rx.recv().await {
                warn!("Received shutdown signal, stop server...");
                srv.stop(true).await
            }
        });
        let err = server.await;
        let title = format!("Server has Stopped!").bold().yellow();
        let time_string = format!(
            "Server running time: {:?}\n",
            self.start_time.unwrap().elapsed()
        )
        .bold()
        .bright_blue();
        warn!("{} \n\n {}", title, time_string);
        err
    }

    pub fn make_logger(s: &Settings) -> Logger {
        let format = &s.logger.actix_log_format;
        let mut logger = match s.prom.exclude_endpoint_log {
            true => Logger::new(format).exclude(&s.prom.endpoint),
            false => Logger::new(format),
        };
        for i in s.logger.exclude_endpoints.iter() {
            logger = logger.exclude(i as &str);
        }
        for i in s.logger.exclude_endpoints_regex.iter() {
            logger = logger.exclude_regex(i as &str);
        }
        logger
    }

    pub fn prom_init(addr: &String, sets: &Settings) -> (PrometheusMetrics, IntCounterVec) {
        // Ready prometheus
        println!(
            "> {}",
            format!("Prometheus endpoint: {}", sets.prom.endpoint).green()
        );
        println!(
            "> {}",
            format!("Prometheus namespace: {}", sets.prom.namespace).green()
        );
        println!(
            "> {}\n",
            format!(
                "Prometheus metrics address: http://{}{}",
                addr, sets.prom.endpoint
            )
            .bold()
            .green()
        );

        // Labels
        let mut labels = std::collections::HashMap::new();
        labels.insert("job".to_string(), sets.prom.namespace.to_string());

        // Counter
        let counter_opts = opts!("counter", "some random counter").namespace("api");
        let counter = IntCounterVec::new(counter_opts, &["endpoint", "method", "status"]).unwrap();

        // Init prometheus
        let prometheus = PrometheusMetrics::new(
            &sets.prom.namespace,
            Some(&sets.prom.endpoint),
            Some(labels),
        );
        prometheus
            .registry
            .register(Box::new(counter.clone()))
            .unwrap();

        (prometheus, counter)
    }
}
