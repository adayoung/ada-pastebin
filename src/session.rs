use std::collections::VecDeque;
use tower_cookies::cookie::SameSite;
use tower_cookies::{Cookie, Cookies};

pub fn update_session(cookies: &Cookies, paste_id: &str) {
    let session = cookies.get("__Secure-_pb_session");

    let mut paste_ids = match session {
        Some(pids) => {
            let session: VecDeque<String> = serde_json::from_str(pids.value()).unwrap_or_default();
            session
        }
        None => VecDeque::new(),
    };

    paste_ids.push_back(paste_id.to_owned());
    if paste_ids.len() > 10 {
        paste_ids.pop_front();
    }

    let paste_ids = serde_json::to_string(&paste_ids).unwrap();
    cookies.add(
        Cookie::build(("__Secure-_pb_session", paste_ids))
            .path("/pastebin/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .into(),
    );
}
