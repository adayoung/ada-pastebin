use askama_axum::Template;
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "base.html.j2")]
pub struct BaseTemplate {
    pub static_domain: String,
}

#[derive(Template)]
#[template(path = "about.html.j2")]
pub struct AboutTemplate {
    pub static_domain: String,
    pub recaptcha_key: String,
}

#[derive(Template)]
#[template(path = "pastebin.html.j2")]
pub struct PastebinTemplate {
    pub static_domain: String,
    pub recaptcha_key: String,
    pub csrf_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct PasteForm {
    pub csrf_token: String,
}
