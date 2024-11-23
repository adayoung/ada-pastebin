use crate::forms;
use crate::runtime;
use crate::s3;
use axum::http::StatusCode;
use bigdecimal::BigDecimal;
use chrono::Utc;
use num_traits::FromPrimitive;
use rand::Rng;
use sqlx::postgres::PgPool;
use sqlx::query;
use sqlx::types::chrono::DateTime;
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

pub async fn new_paste(
    state: &runtime::AppState,
    form: &forms::PasteForm,
    score: f64,
) -> Result<String, (StatusCode, String)> {
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

enum PasteFormat {
    Text(String),
    Html(String),
}

struct Paste {
    paste_id: String,
    user_id: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    format: PasteFormat,
    date: DateTime<Utc>,
    gdriveid: Option<String>, // Googe Drive object ID
    rcscore: BigDecimal,      // Recaptcha score
}

impl Paste {
    fn new(form: &forms::PasteForm, score: f64) -> Result<Self, (StatusCode, String)> {
        // Limit title to 50 characters only
        let mut title = form.title.clone().unwrap_or_default();
        if title.len() > 50 {
            title = title.chars().take(50).collect();
        }

        // Limit tags to 15 of no more than 15 alphanumeric each
        let tags = form
            .tags
            .clone()
            .unwrap_or_default()
            .split_whitespace()
            .map(|part| {
                part.chars()
                    .filter(|x| char::is_alphanumeric(*x))
                    .take(15)
                    .collect()
            })
            .map(|word: String| word.to_lowercase())
            .filter(|word: &String| word.len() > 0)
            .take(15)
            .collect();

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
            tags: Some(tags),
            format,
            date: now,
            gdriveid: None, // TODO: Get Google Drive ID if available
            rcscore,
        };

        Ok(paste)
    }

    async fn save(
        &self,
        db: &PgPool,
        s3_bucket: &String,
        s3_prefix: &String,
        content: &String,
    ) -> Result<String, String> {
        // Convert rust types to SQLx types
        let tags: Option<&[String]> = self.tags.as_ref().map(|vec| vec.as_slice());

        let format = match self.format {
            PasteFormat::Text(ref text) => text,
            PasteFormat::Html(ref html) => html,
        };

        // Determine file extension and content type for S3
        let ext = match self.format {
            PasteFormat::Text(_) => "txt",
            PasteFormat::Html(_) => "html",
        };

        let content_type = match self.format {
            PasteFormat::Text(_) => "text/plain".to_string(),
            PasteFormat::Html(_) => "text/html".to_string(),
        };

        // Start a DB transaction
        let mut transaction = match db.begin().await {
            Ok(transaction) => transaction,
            Err(err) => return Err(format!("Failed to start transaction: {}", err)),
        };

        query!(
            r#"
            INSERT INTO pastebin (paste_id, user_id, title, tags, format, date, gdriveid, rcscore)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            self.paste_id,
            self.user_id,
            self.title,
            tags,
            format,
            self.date,
            self.gdriveid,
            self.rcscore
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| format!("Failed to insert paste: {}", err))?;

        // Upload to S3 while in transaction
        match s3::upload(
            &s3_bucket,
            &format!("{}{}.{}", s3_prefix, self.paste_id, ext),
            &content,
            &content_type,
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
                Ok(_) => return Err(format!("Failed to upload to S3: {}", err)),
                Err(err) => {
                    error!("Failed to rollback transaction: {}", err);
                    Err(format!("Failed to rollback transaction: {}", err))
                }
            },
        }
    }
}
