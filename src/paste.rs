use crate::forms;
use crate::runtime;
use crate::s3;
use crate::utils;
use axum::http::StatusCode;
use bigdecimal::BigDecimal;
use chrono::Utc;
use num_traits::FromPrimitive;
use rand::Rng;
use serde::Serialize;
use sqlx::postgres::PgPool;
use sqlx::types::chrono::DateTime;
use sqlx::Error::RowNotFound;
use sqlx::{query, query_as, FromRow};
use tokio::time::{sleep, Duration};
use tracing::error;

fn generate_paste_id() -> String {
    let all_characters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let alphanumeric = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    let mut id = String::new();
    let mut index: usize;

    for _ in 0..7 {
        index = rng.gen_range(0..all_characters.len());
        id.push(all_characters.chars().nth(index).unwrap());
    }

    // Ensure we don't end up with a weird character in the end
    index = rng.gen_range(0..alphanumeric.len());
    id.push(all_characters.chars().nth(index).unwrap());

    id
}

pub fn fix_tags(tags: &Option<String>) -> Vec<String> {
    // Limit tags to 15 of no more than 15 alphanumeric each
    let tags: Vec<String> = tags
        .clone()
        .unwrap_or_default()
        .split_whitespace()
        .map(|tag| {
            tag.chars()
                .filter(|x| char::is_alphanumeric(*x))
                .take(15)
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|tag: &String| !tag.is_empty())
        .collect();
    tags
}

pub async fn new_paste(
    state: &runtime::AppState,
    form: &forms::PasteForm,
    score: f64,
) -> Result<String, (StatusCode, String)> {
    #[cfg(not(debug_assertions))]
    {
        if score < 0.7 {
            return Err((
                StatusCode::FORBIDDEN,
                "Oop, bot check failed! This site is for humans!".to_string(),
            ));
        }
    }

    let paste = match Paste::new(form, score) {
        Ok(paste) => paste,
        Err(err) => return Err(err),
    };

    match paste
        .save(
            &state.db,
            &state.config.s3_bucket,
            &state.config.s3_prefix,
            &form.content,
        )
        .await
    {
        Ok(paste_id) => Ok(paste_id),
        Err(err) => {
            error!("Failed to save paste: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Meep! We couldn't save that paste :-(".to_string(),
            ))
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PasteFormat {
    Text(String),
    Html(String),
}

impl From<String> for PasteFormat {
    fn from(format: String) -> Self {
        match format.as_str() {
            "html" => PasteFormat::Html(format),
            _ => PasteFormat::Text(format),
        }
    }
}

#[derive(FromRow)]
pub struct Paste {
    pub paste_id: String,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub format: PasteFormat,
    pub date: DateTime<Utc>,
    pub gdriveid: Option<String>, // Googe Drive object ID
    pub gdrivedl: Option<String>, // Google Drive download URL
    pub s3_key: String,
    pub rcscore: BigDecimal, // Recaptcha score
    pub views: i64,
    pub last_seen: DateTime<Utc>,
}

// Used for DELETE /pastes/:paste_id
struct DeletePaste {
    pub s3_key: String,
}

#[derive(FromRow, Serialize)]
pub struct SearchPaste {
    pub paste_id: String,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub format: PasteFormat,
    pub date: DateTime<Utc>,
    pub views: i64,
}

impl Paste {
    fn new(form: &forms::PasteForm, score: f64) -> Result<Self, (StatusCode, String)> {
        // Limit title to 50 characters only
        let mut title = form.title.clone().unwrap_or_default();
        if title.len() > 50 {
            title = title.chars().take(50).collect();
        }

        let tags = fix_tags(&form.tags);

        // We want unique tags with their order preserved so no HashSet
        let mut unique_tags: Vec<String> = vec![];
        for tag in tags.iter() {
            if !unique_tags.contains(tag) {
                unique_tags.push(tag.clone());
            }

            if unique_tags.len() > 15 {
                break;
            }
        }

        let format = match form.format.as_str() {
            "plain" => PasteFormat::Text(form.format.clone()),
            "html" => PasteFormat::Html(form.format.clone()),
            _ => return Err((StatusCode::BAD_REQUEST, "Invalid format".to_string())),
        };

        let rcscore = match BigDecimal::from_f64(score) {
            Some(score) => score,
            None => return Err((StatusCode::BAD_REQUEST, "Invalid score".to_string())),
        };

        let now = Utc::now();
        let paste_id = generate_paste_id(); // FIXME: Check for duplicates before using
        let paste = Paste {
            paste_id: paste_id.clone(),
            user_id: None, // TODO: Get user ID once we have an auth system
            title: Some(title),
            tags: Some(unique_tags),
            format,
            date: now,
            gdriveid: None, // TODO: Get Google Drive ID if available
            gdrivedl: None, // TODO: Get Google Drive download URL if available
            s3_key: "".to_string(),
            rcscore,
            views: 0,
            last_seen: now,
        };

        Ok(paste)
    }

    async fn save(
        &self,
        db: &PgPool,
        s3_bucket: &str,
        s3_prefix: &str,
        content: &str,
    ) -> Result<String, String> {
        // Convert rust types to SQLx types
        let tags: Option<&[String]> = self.tags.as_deref();

        let format = match self.format {
            PasteFormat::Text(ref text) => text,
            PasteFormat::Html(ref html) => html,
        };

        // Determine file extension for S3
        let ext = match self.format {
            PasteFormat::Text(_) => "txt",
            PasteFormat::Html(_) => "html",
        };

        // Determine content type for S3
        let content_type = match self.format {
            PasteFormat::Text(_) => "text/plain".to_string(),
            PasteFormat::Html(_) => "text/html".to_string(),
        };

        // Crunch crunch!
        let (s3_content, content_encoding) = match utils::compress(content).await {
            Ok(response) => response,
            Err(err) => return Err(format!("Failed to compress content: {}", err)),
        };

        // Let's append .br to the S3 key if we're using brotli compression
        let mut s3_key = format!("{}{}.{}", s3_prefix, self.paste_id, ext);
        if content_encoding == "br" {
            s3_key.push_str(".br");
        }

        let content_length = s3_content.len() as i32;

        // Start a DB transaction
        let mut transaction = match db.begin().await {
            Ok(transaction) => transaction,
            Err(err) => return Err(format!("Failed to start transaction: {}", err)),
        };

        query!(
            r#"
            INSERT INTO pastebin (paste_id, user_id, title, tags, format, date, gdriveid, gdrivedl, s3_key, s3_content_length, rcscore, last_seen)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            self.paste_id,
            self.user_id,
            self.title,
            tags,
            format,
            self.date,
            self.gdriveid,
            self.gdrivedl,
            s3_key,
            content_length,
            self.rcscore,
            self.last_seen
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| format!("Failed to insert paste: {}", err))?;

        match s3::upload(
            s3_bucket,
            &s3_key,
            s3_content,
            &content_type,
            &content_encoding,
            &self.title,
            &self.tags,
            &format!("{}.{}", self.paste_id, ext),
        )
        .await
        {
            Ok(_) => match transaction.commit().await {
                Ok(_) => Ok(self.paste_id.clone()),
                Err(err) => {
                    error!("Failed to commit transaction: {}", err);
                    Err(format!("Failed to commit transaction: {}", err))
                }
            },
            Err(err) => match transaction.rollback().await {
                Ok(_) => Err(format!("Failed to upload to S3: {}", err)),
                Err(err) => {
                    error!("Failed to rollback transaction: {}", err);
                    Err(format!("Failed to rollback transaction: {}", err))
                }
            },
        }
    }

    pub async fn get(db: &PgPool, paste_id: &String) -> Result<Paste, (StatusCode, String)> {
        let paste = match query_as!(
            Paste,
            r#"
                SELECT paste_id, user_id, title, tags, format, date, gdriveid, gdrivedl, s3_key, rcscore, views, last_seen
                FROM pastebin
                WHERE paste_id = $1
                "#,
            paste_id
        )
        .fetch_one(db)
        .await
        {
            Ok(paste) => paste,
            Err(err) => match err {
                RowNotFound => {
                    return Err((StatusCode::NOT_FOUND, "Paste not found".to_string()));
                }
                _ => {
                    error!("Failed to fetch paste: {}", err);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to fetch paste: {}", err),
                    ));
                }
            },
        };

        Ok(paste)
    }

    pub async fn delete(
        db: &PgPool,
        s3_bucket: &str,
        paste_id: &str,
    ) -> Result<String, (StatusCode, String)> {
        let mut transaction = match db.begin().await {
            Ok(transaction) => transaction,
            Err(err) => {
                error!("Failed to start transaction: {}", err);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to start transaction: {}", err),
                ));
            }
        };

        let paste = query_as!(
            DeletePaste,
            r#"
            WITH paste AS (
                DELETE FROM pastebin
                WHERE paste_id = $1
                RETURNING s3_key
            )
            SELECT s3_key FROM paste
            "#,
            paste_id
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete paste: {}", err),
            )
        })?;

        match s3::delete(s3_bucket, &paste.s3_key).await {
            Ok(()) => match transaction.commit().await {
                Ok(_) => Ok(paste.s3_key),
                Err(err) => {
                    error!("Failed to commit transaction: {}", err);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to commit transaction: {}", err),
                    ))
                }
            },
            Err(err) => match transaction.rollback().await {
                Ok(_) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to delete from S3: {}", err),
                )),
                Err(err) => {
                    error!("Failed to commit transaction: {}", err);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to commit transaction: {}", err),
                    ))
                }
            },
        }
    }

    pub async fn search(
        db: &PgPool,
        tags: &Vec<String>,
        page: i64,
    ) -> Result<Vec<SearchPaste>, String> {
        let pastes = match query_as!(
            SearchPaste,
            "
            SELECT paste_id, title, tags, format, date, views
            FROM pastebin
            WHERE
                tags @> $1::varchar[]
            ORDER BY date DESC
            LIMIT 10
            OFFSET $2
            ",
            tags,
            (page - 1) * 10
        )
        .fetch_all(db)
        .await
        {
            Ok(pastes) => pastes,
            Err(err) => {
                error!("Failed to search pastes: {}", err);
                return Err(format!("Failed to search pastes: {}", err));
            }
        };

        Ok(pastes)
    }

    pub fn get_content_url(&self, s3_bucket_url: &str) -> String {
        if self.gdrivedl.is_none() {
            format!("{}{}", s3_bucket_url, self.s3_key)
        } else {
            format!("/pastebinc/{}/content", self.paste_id)
        }
    }

    pub fn get_title(&self) -> String {
        if self.title == Some("".to_string()) {
            self.paste_id.clone()
        } else {
            self.title.clone().unwrap_or(self.paste_id.clone())
        }
    }

    pub fn get_format(&self) -> String {
        match self.format {
            PasteFormat::Text(_) => "plain".to_string(),
            PasteFormat::Html(_) => "html".to_string(),
        }
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }

    pub fn get_views(&self, state: &runtime::AppState) -> u64 {
        *state
            .counter
            .entry(self.paste_id.clone())
            .and_modify(|counter| *counter += 1)
            .or_insert_with(|| self.views as u64 + 1)
    }

    pub async fn save_views(&self, db: &PgPool, views: i64) {
        match query!(
            r#"
            UPDATE pastebin
            SET views = $1
            WHERE paste_id = $2
            "#,
            views,
            self.paste_id
        )
        .execute(db)
        .await
        {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to save views: {}", err);
            }
        }
    }
}

pub async fn update_views(state: &runtime::AppState, do_sleep: bool) {
    loop {
        if do_sleep {
            sleep(Duration::from_secs(state.config.update_views_interval)).await;
        }

        for entry in state.counter.iter() {
            let paste_id = entry.key().clone();
            let views = *entry.value();

            let paste_result = Paste::get(&state.db, &paste_id).await;

            match paste_result {
                Ok(paste) => {
                    paste.save_views(&state.db, views as i64).await;
                }
                Err(err) => {
                    if err.0 != StatusCode::NOT_FOUND {
                        error!("Failed to fetch paste: {:?}", err);
                    }
                }
            }
        }

        state.counter.clear();
        if !do_sleep {
            break;
        }
    }
}
