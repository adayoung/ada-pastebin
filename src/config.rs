use config::{Config, Environment};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    pub static_domain: String,
    pub bind_addr: String,
    pub port: u16,
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

        // Override with environment variables
        config = config.add_source(Environment::with_prefix("APP"));

        // Build the config
        let config = match config.build() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error loading config: {}", err);
                std::process::exit(1);
            }
        };

        // Deserialize the config
        match config.try_deserialize() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error deserializing config: {}", err);
                std::process::exit(1);
            }
        }
    }
}
