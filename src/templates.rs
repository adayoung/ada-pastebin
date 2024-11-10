use askama_axum::Template;

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub static_domain: String,
}
