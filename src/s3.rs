use aws_sdk_s3 as s3;
use tracing::error;

#[allow(clippy::too_many_arguments)]
pub async fn upload(
    bucket: &str,
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

    let _config = aws_config::load_from_env().await;
    let config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);
    match client
        .put_object()
        .bucket(bucket)
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
            return Err(format!("{}", err));
        }
    };

    Ok(())
}

pub async fn delete(bucket: &str, key: &str) -> Result<(), String> {
    let _config = aws_config::load_from_env().await;

    let config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);

    match client.delete_object().bucket(bucket).key(key).send().await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to delete from S3: {}", err);
            return Err(format!("{}", err));
        }
    };

    Ok(())
}
