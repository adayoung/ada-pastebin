use crate::paste::PasteFormat;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValidDestination {
    DataStore,
    GDrive,
}

#[derive(Deserialize)]
pub struct PasteForm {
    pub csrf_token: String,
    pub token: String,

    pub content: String,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub format: PasteFormat,
    pub destination: ValidDestination,
}

#[derive(Deserialize)]
pub struct PasteDeleteForm {
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct PasteAPIForm {
    pub content: String,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub format: PasteFormat,
}

#[derive(Deserialize)]
pub struct PasteAPIDeleteForm {
    pub paste_id: String,
}
