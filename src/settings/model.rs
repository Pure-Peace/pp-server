use std::env;

use crate::constants::BANNER;
use crate::utils;
use colored::Colorize;
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LocalConfig {
    pub env: String,
    pub cfg: config::Config,
    pub data: Settings,
}

impl LocalConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        // Print banner
        println!("{}", BANNER.red());

        // Start loading
        println!("{}", "> Start loading settings!".green());
        let env = Settings::load_env();
        let cfg = Settings::load_settings(env.clone())?;
        let data: Settings = cfg.clone().try_into()?;
        utils::checking_osu_dir(&data);
        println!(
            "{}",
            "> Configuration loaded successfully!\n".bold().green()
        );
        // You can deserialize (and thus freeze) the entire configuration as cfg.try_into()
        Ok(LocalConfig { env, cfg, data })
    }

    #[inline(always)]
    pub fn init() -> Self {
        let cfg = LocalConfig::new();
        if let Err(err) = cfg {
            error!("Settings failed to initialize, please check the local configuration file! Error: {:?}", err);
            panic!();
        }
        cfg.unwrap()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[cfg(feature = "peace")]
    pub postgres: deadpool_postgres::Config,
    #[cfg(feature = "peace")]
    pub redis: deadpool_redis::Config,
    #[cfg(not(feature = "peace"))]
    pub osu_api_keys: Vec<String>,

    #[cfg(feature = "peace")]
    pub check_pools_on_created: bool,
    #[cfg(feature = "peace")]
    pub check_db_version_on_created: bool,

    pub env: String,
    pub debug: bool,
    pub osu_files_dir: String,
    pub recalculate_osu_file_md5: bool,
    pub preload_osu_files: bool,
    pub beatmap_cache_max: i32,
    pub beatmap_cache_timeout: u64,
    pub auto_clean_cache: bool,
    pub auto_clean_interval: u64,
    pub auto_pp_recalculate: AutoPPRecalculate,
    pub server: Server,
    pub logger: Logger,
    #[serde(rename = "prometheus")]
    pub prom: Prometheus,
}

impl Settings {
    pub fn load_env() -> String {
        // Load .env
        dotenv().ok();
        // Current env
        // Default to 'development' env
        // Args > .env file
        let env = match env::args().nth(1) {
            None => env::var("RUN_MODE").unwrap_or_else(|_| "development".into()),
            Some(any) => any,
        };
        println!(
            "{}",
            format!("> Current environment: {}", env.bold().yellow()).green()
        );
        env
    }

    pub fn load_settings(env: String) -> Result<config::Config, config::ConfigError> {
        let mut cfg = config::Config::new();

        // Set the env
        cfg.set("env", env.clone())?;
        println!("{}", "> Loading user settings...".green());

        // The "default" configuration file
        cfg.merge(config::File::with_name("pp-server-config/default"))?;

        // Add in the current environment file
        cfg.merge(config::File::with_name(&format!("pp-server-config/{}", env)).required(true))
            .expect(
                "Please make sure that the configuration file of the current environment exists",
            );

        // Initial logger
        println!("{}", "> Initializing logger...".green());
        Logger::init(&cfg);

        // Set the server addr
        let server: &[String; 2] = &[cfg.get("server.host")?, cfg.get("server.port")?];
        cfg.set("server.addr", format!("{}:{}", server[0], server[1]))?;

        // Example: Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        // cfg.merge(Environment::with_prefix("app"))?;

        Ok(cfg)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggerMode {
    debug: String,
    error: String,
    warn: String,
    info: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Logger {
    pub level: String,
    pub mode: LoggerMode,
    pub actix_log_format: String,
    pub exclude_endpoints: Vec<String>,
    pub exclude_endpoints_regex: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Prometheus {
    pub namespace: String,
    pub endpoint: String,
    pub exclude_endpoint_log: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AutoPPRecalculate {
    pub interval: u64,
    pub max_retry: i32,
}
