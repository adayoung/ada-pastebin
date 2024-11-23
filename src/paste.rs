use crate::forms;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::postgres::PgPool;

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
    db: &PgPool,
    form: &forms::PasteForm,
    score: f64,
) -> Result<String, (StatusCode, String)> {
    let paste = Paste::new(form, score);

    #[cfg(debug_assertions)]
    {
        println!("{:#?}", paste);
    }

    Ok(paste.paste_id)
}

#[derive(Debug)]
enum PasteFormat {
    Text(String),
    Html(String),
}

#[derive(Debug)]
struct Paste {
    paste_id: String,
    user_id: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    format: PasteFormat,
    date: DateTime<Utc>,
    gdriveid: Option<String>, // Googe Drive object ID
    rcscore: f64,             // Recaptcha score
}

impl Paste {
    fn new(form: &forms::PasteForm, score: f64) -> Self {
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

        let now = Utc::now();
        let paste_id = generate_paste_id();
        let paste = Paste {
            paste_id: paste_id.clone(),
            user_id: None, // TODO: Get user ID once we have an auth system
            title: Some(title),
            tags: Some(tags),
            format: PasteFormat::Text(form.format.clone()),
            date: now,
            gdriveid: None, // TODO: Get Google Drive ID if available
            rcscore: score,
        };

        paste
    }
}
