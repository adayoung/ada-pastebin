use crate::forms;
use axum::http;
use rand::Rng;

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
    form: forms::PasteForm,
    _score: f64,
) -> Result<String, (http::status::StatusCode, String)> {
    let paste_id = generate_paste_id();

    #[cfg(debug_assertions)]
    {
        println!("{:#?}", form);
    }

    Ok(paste_id)
}
