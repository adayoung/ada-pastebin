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

    pub cloudflare_api_key: String,
    pub cloudflare_purge_url: String,
    pub cloudflare_enabled: bool,

    pub cookie_key: String,
    pub cookie_salt: String,
    pub cookie_secure: bool,
    pub update_views_interval: u64,

    pub s3_bucket_url: String,
    pub s3_bucket: String,
    pub s3_prefix: String,

    pub aws_region: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub aws_endpoint: String,
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

        // config = config.set_default("cloudflare_api_key", "").unwrap();
        // config = config.set_default("cloudflare_purge_url", "").unwrap();
        config = config.set_default("cloudflare_enabled", true).unwrap();

        // config = config.set_default("cookie_key", "-meow-meow-").unwrap();
        // config = config.set_default("cookie_salt", "-bork-bork-").unwrap();
        config = config.set_default("csrf_secure_cookie", true).unwrap();
        config = config.set_default("update_views_interval", 300).unwrap();

        config = config
            .set_default("s3_bucket_url", "https://bin.ada-young.com/")
            .unwrap();
        config = config
            .set_default("s3_bucket", "bin.ada-young.com")
            .unwrap();
        config = config.set_default("s3_prefix", "content/").unwrap();

        config = config.set_default("aws_region", "us-east-1").unwrap();
        config = config.set_default("aws_access_key_id", "").unwrap();
        config = config.set_default("aws_secret_access_key", "").unwrap();
        config = config
            .set_default("aws_endpoint", "s3.amazonaws.com")
            .unwrap();

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
