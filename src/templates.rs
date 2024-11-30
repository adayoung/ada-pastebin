use crate::paste::Paste;
use askama_axum::Template;

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
}

#[derive(Template)]
#[template(path = "pastebin.html.j2")]
pub struct PastebinTemplate {
    pub static_domain: String,
    pub recaptcha_key: String,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "paste.html.j2")]
pub struct PasteTemplate {
    pub static_domain: String,
    pub s3_bucket_url: String,
    pub csrf_token: String,
    pub paste: Paste,
    pub views: u64,
    pub owned: bool,
}
