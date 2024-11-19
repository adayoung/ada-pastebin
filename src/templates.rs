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
#[template(path = "pastebin.html.j2")]
pub struct PastebinTemplate {
    pub static_domain: String,
    pub recaptcha_key: String,
}
