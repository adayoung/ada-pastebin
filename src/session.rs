use std::collections::VecDeque;
use tower_cookies::cookie::SameSite;
use tower_cookies::{Cookie, Cookies, Key};

pub fn update_session(key: &Key, cookies: &Cookies, paste_id: &str) {
    let mut paste_ids = get_session(key, cookies);

    paste_ids.push_back(paste_id.to_owned());
    if paste_ids.len() > 10 {
        paste_ids.pop_front();
    }

    let paste_ids = serde_json::to_string(&paste_ids).unwrap();

    let cookies = cookies.private(key);
    cookies.add(
        Cookie::build(("__Secure-_pb_session", paste_ids))
            .path("/pastebin/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .into(),
    );
}

fn get_session(key: &Key, cookies: &Cookies) -> VecDeque<String> {
    let cookies = cookies.private(key);
    let session = cookies.get("__Secure-_pb_session");

    let paste_ids = match session {
        Some(pids) => {
            let session: VecDeque<String> = serde_json::from_str(pids.value()).unwrap_or_default();
            session
        }
        None => VecDeque::new(),
    };
    paste_ids
}

pub fn is_paste_in_session(key: &Key, cookies: &Cookies, paste_id: &str) -> bool {
    let paste_ids = get_session(key, cookies);
    paste_ids.contains(&paste_id.to_owned())
}
