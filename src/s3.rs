use aws_sdk_s3 as s3;
use brotli::CompressorWriter;
use std::io::Write;
use tracing::error;

pub async fn upload(
    bucket: &String,
    key: &String,
    content: &String,
    content_type: &String,
    title: &Option<String>,
    paste_id_w_ext: &String,
) -> Result<(String, i32), String> {
    let _config = aws_config::load_from_env().await;

    let config = s3::Config::from(&_config)
        .to_builder()
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);

    let mut real_key = key.clone();
    let body: Vec<u8>;
    let content_encoding: String;
    match compress(content) {
        Ok(result) => {
            body = result.0;
            content_encoding = result.1;
            if content_encoding == "br" {
                real_key.push_str(".br");
            }
        }
        Err(err) => {
            return Err(format!("{}", err));
        }
    };
    let content_length = body.len();

    let title = match title {
        Some(title) => title,
        None => "",
    };

    match client
        .put_object()
        .bucket(bucket)
        .key(&real_key)
        .body(body.into())
        .content_type(content_type)
        .content_encoding(content_encoding)
        .content_disposition(format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            paste_id_w_ext, paste_id_w_ext
        ))
        .metadata("title", title)
        .send()
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to upload to S3: {}", err);
            return Err(format!("{}", err));
        }
    };

    Ok((real_key, content_length.try_into().unwrap()))
}

fn compress(content: &String) -> Result<(Vec<u8>, String), String> {
    if content.len() < 1024 {
        return Ok((content.as_bytes().to_vec(), "identity".to_string()));
    }

    let mut encoder = CompressorWriter::new(Vec::new(), 4096, 6, 22);
    match encoder.write_all(content.as_bytes()) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to compress content: {}", err);
            return Err(format!("{}", err));
        }
    };

    match encoder.flush() {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to compress content: {}", err);
            return Err(format!("{}", err));
        }
    };

    Ok((encoder.into_inner(), "br".to_string()))
}
