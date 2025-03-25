use crate::runtime;
use aws_credential_types::Credentials;
use aws_sdk_s3 as s3;
use std::sync::OnceLock;
use tracing::error;

static S3_CLIENT: OnceLock<s3::Client> = OnceLock::new();

pub async fn init_s3_client(state: &runtime::AppState) {
    let _config = aws_config::defaults(aws_config::BehaviorVersion::v2025_01_17())
        .region(aws_config::Region::new(state.config.aws_region.clone()))
        .endpoint_url(&state.config.aws_endpoint)
        .credentials_provider(Credentials::new(
            state.config.aws_access_key_id.clone(),
            state.config.aws_secret_access_key.clone(),
            None,
            None,
            "custom",
        ))
        .load()
        .await;

    let s3_config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(s3_config);

    // This will panic if called more than once!
    S3_CLIENT.set(client).unwrap();
}

fn get_client() -> &'static s3::Client {
    S3_CLIENT.get().expect("S3 client not initialized")
}

#[allow(clippy::too_many_arguments)]
pub async fn upload(
    state: &runtime::AppState,
    key: &str,
    content: Vec<u8>,
    content_type: &str,
    content_encoding: &str,
    title: &Option<String>,
    tags: &Option<Vec<String>>,
    filename: &str,
    fake_it: bool,
) -> Result<(), String> {
    if fake_it {
        return Ok(());
    }

    let title = match title {
        Some(title) => title,
        None => "",
    };

    let tags = tags
        .as_ref()
        .map(|tags| tags.join(", "))
        .unwrap_or_default();

    let content_length = content.len() as i64;
    let filename: String = filename.chars().filter(|c| c != &'~').collect();
    match get_client()
        .put_object()
        .bucket(state.config.s3_bucket.clone())
        .key(key)
        .body(content.into())
        .content_type(content_type)
        .content_encoding(content_encoding)
        .content_disposition(format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            filename, filename
        ))
        .content_length(content_length)
        .metadata("title", title)
        .metadata("tags", tags)
        .send()
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to upload to S3: {}", err);
            return Err(err.to_string());
        }
    };

    Ok(())
}

pub async fn delete(state: &runtime::AppState, key: &str, fake_it: bool) -> Result<(), String> {
    if fake_it {
        return Ok(());
    }

    match get_client()
        .delete_object()
        .bucket(state.config.s3_bucket.clone())
        .key(key)
        .send()
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to delete from S3: {}", err);
            return Err(err.to_string());
        }
    };

    Ok(())
}
