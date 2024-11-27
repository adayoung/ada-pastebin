use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)] // TODO: Remove Serialize
pub struct PasteForm {
    pub csrf_token: String,
    pub token: String,

    pub content: String,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub format: String,
    pub destination: String,
}

#[derive(Deserialize)]
pub struct PasteDeleteForm {
    pub csrf_token: String,
}
