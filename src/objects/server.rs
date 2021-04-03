use actix_cors::Cors;
use actix_web::{dev::Server, middleware::Logger, web::Data, App, HttpServer};
use async_std::channel::{unbounded, Receiver, Sender};
use std::time::Instant;

use colored::Colorize;

use actix_web_prom::PrometheusMetrics;
use prometheus::{opts, IntCounterVec};

use crate::settings::model::{LocalConfig, Settings};
use crate::{caches::Caches, renders::MainPage};
use crate::{routes, utils};

pub struct PPserver {
    pub addr: String,
    pub local_config: LocalConfig,
    pub caches: Data<Caches>,
    pub prometheus: PrometheusMetrics,
    pub counter: IntCounterVec,
    pub server: Option<Server>,
    pub sender: Sender<Option<Server>>,
    pub receiver: Receiver<Option<Server>>,
    pub start_time: Option<Instant>,
}

impl PPserver {
    pub fn new(local_config: LocalConfig) -> Self {
        let sets = &local_config.data;
        let addr = local_config
            .cfg
            .get("server.addr")
            .unwrap_or("127.0.0.1:8088".to_string());

        // Prometheus
        let (prometheus, counter) = Self::prom_init(&addr, sets);
        let (sender, receiver) = unbounded();

        let caches = Data::new(Caches::new());

        Self {
            addr,
            local_config,
            caches,
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
            let settings_cloned = self.local_config.data.clone();
            let counter = self.counter.clone();
            let caches = self.caches.clone();
            let sender = Data::new(self.sender.clone());
            let prom = self.prometheus.clone();
            let default_render = MainPage::new();
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
                    .app_data(caches.clone())
                    .data(settings_cloned.clone())
                    .data(counter.clone())
                    .data(default_render.clone())
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
        // Should preload or not
        if self.local_config.data.preload_osu_files {
            utils::preload_osu_files(&self.local_config.data.osu_files_dir, &self.caches).await;
        };
        self.run_server().await;
        // Wait for stopped
        self.stopped().await
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
