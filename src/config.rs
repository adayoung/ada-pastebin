use config::{Config, Environment};
use serde::Deserialize;
use tracing::{error, info};

#[derive(Deserialize)]
pub struct AppConfig {
    pub static_domain: String,
    pub bind_addr: String,
    pub port: u16,

    pub recaptcha_key: String,
    pub recaptcha_secret: String,

    pub csrf_secure_cookie: bool,
}

impl AppConfig {
    pub fn new() -> Self {
        let mut config = Config::builder();

        // Set application defaults
        config = config
            .set_default("static_domain", "localhost:2024")
            .unwrap();
        config = config.set_default("bind_addr", "127.0.0.1").unwrap();
        config = config.set_default("port", 2024).unwrap();

        // config = config.set_default("recaptcha_key", "").unwrap();
        // config = config.set_default("recaptcha_secret", "").unwrap();

        config = config.set_default("csrf_secure_cookie", true).unwrap();

        // Check for the presence of a config.toml file and use it
        if std::path::Path::new("config.toml").is_file() {
            info!("Found config.toml, using it!");
            config = config.add_source(config::File::with_name("config"));
        } else {
            info!("No config.toml found, using defaults!");
        }

        // Override with environment variables
        config = config.add_source(Environment::with_prefix("APP"));

        // Build the config
        let config = match config.build() {
            Ok(config) => config,
            Err(err) => {
                error!("Error loading config: {}", err);
                std::process::exit(1);
            }
        };

        // Deserialize the config
        match config.try_deserialize() {
            Ok(config) => config,
            Err(err) => {
                error!("Error deserializing config: {}", err);
                std::process::exit(1);
            }
        }
    }
}
