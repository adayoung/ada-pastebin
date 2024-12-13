use crate::runtime;
use aws_credential_types::Credentials;
use aws_sdk_s3 as s3;
use tracing::error;

async fn build_aws_config(state: &runtime::AppState) -> aws_config::SdkConfig {
    aws_config::defaults(aws_config::BehaviorVersion::v2024_03_28())
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
        .await
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
    paste_id_w_ext: &str,
) -> Result<(), String> {
    let title = match title {
        Some(title) => title,
        None => "",
    };

    let tags = tags
        .as_ref()
        .map(|tags| tags.join(", "))
        .unwrap_or_default();

    let content_length = content.len() as i64;
    let paste_id_w_ext: String = paste_id_w_ext.chars().filter(|c| c != &'~').collect();

    let _config = build_aws_config(state).await;

    let config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);
    match client
        .put_object()
        .bucket(state.config.s3_bucket.clone())
        .key(key)
        .body(content.into())
        .content_type(content_type)
        .content_encoding(content_encoding)
        .content_disposition(format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            paste_id_w_ext, paste_id_w_ext
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

    let _config = build_aws_config(state).await;

    let config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);

    match client
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
