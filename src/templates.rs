use crate::paste::Paste;
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response}
};
use tracing::error;

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                error!("Failed to render template. Error: {err}");
                (
                StatusCode::INTERNAL_SERVER_ERROR,
                "We failed to render a template D:",
            )
                .into_response()
            },
        }
    }
}

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
    pub user_id: Option<String>,
}

#[derive(Template)]
#[template(path = "pastebin.html.j2")]
pub struct PastebinTemplate {
    pub static_domain: String,
    pub recaptcha_key: String,
    pub csrf_token: String,
    pub user_id: Option<String>,
}

#[derive(Template)]
#[template(path = "paste.html.j2")]
pub struct PasteTemplate {
    pub static_domain: String,
    pub content_url: String,
    pub csrf_token: String,
    pub user_id: Option<String>,
    pub paste: Paste,
    pub owned: bool,
}

#[derive(Template)]
#[template(path = "search.html.j2")]
pub struct SearchTemplate {
    pub static_domain: String,
    pub user_id: Option<String>,
}

#[derive(Template)]
#[template(path = "api-about.html.j2")]
pub struct APIAboutTemplate {
    pub static_domain: String,
    pub user_id: Option<String>,
    pub api_key: String,
}

#[derive(Template)]
#[template(path = "gdrive.html.j2")]
pub struct GDriveTemplate {
    pub result: String,
}
