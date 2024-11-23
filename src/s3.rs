use aws_sdk_s3 as s3;
use tracing::error;

pub async fn upload(
    bucket: &String,
    key: &String,
    content: &String,
    content_type: &String,
) -> Result<(), String> {
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
        .body(content.as_bytes().to_vec().into())
        .content_type(content_type)
        // .content_encoding("br") // TODO: Brotli the content first
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
