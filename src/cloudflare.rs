use crate::runtime;
use scc::HashSet;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::sync::OnceLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

static PURGE_QUEUE: OnceLock<HashSet<String>> = OnceLock::new();
pub fn queue() -> &'static HashSet<String> {
    PURGE_QUEUE.get_or_init(|| HashSet::with_capacity(20))
}

static CF_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    CF_CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn purge_cache(state: &runtime::AppState, now: bool) {
    if queue().len() >= 10 || now {
        if queue().is_empty() {
            return;
        }

        info!("About to purge {} object(s) from Cloudflare cache..", queue().len());

        if !state.config.cloudflare_enabled {
            queue().clear();
            return;
        }

        let mut urls: Vec<String> = Vec::new();
        queue().scan(|key| {
            urls.push(format!("{}{}", state.config.s3_bucket_url, key.clone()));
        });

        let mut request_data = std::collections::HashMap::new();
        request_data.insert("files", urls);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", state.config.cloudflare_api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        match get_client()
            .post(&state.config.cloudflare_purge_url)
            .headers(headers)
            .json(&request_data)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    error!(
                        "Failed to purge cloudflare cache: {}",
                        response.text().await.unwrap()
                    );
                    // TODO: Requeue failed s3_keys again for cache purge
                }
            }
            Err(err) => error!("Failed to purge cloudflare cache: {}", err),
        };

        queue().clear();
    }
}

pub async fn cleanup_cache(state: &runtime::AppState, do_sleep: bool, now: bool) {
    loop {
        if do_sleep {
            sleep(Duration::from_secs(3600)).await;
        }

        purge_cache(state, now).await;

        if !do_sleep {
            break;
        }
    }
}
