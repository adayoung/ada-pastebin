use std::collections::VecDeque;
use tower_cookies::{Cookie, Cookies};

pub fn update_session(cookies: &Cookies, paste_id: &String) {
    let session = cookies.get("_pb_session");

    let mut paste_ids = match session {
        Some(pids) => {
            let session: VecDeque<String> = match serde_json::from_str(pids.value()) {
                Ok(session) => session,
                Err(_) => VecDeque::new(),
            };
            session
        }
        None => VecDeque::new(),
    };

    paste_ids.push_back(paste_id.clone());
    if paste_ids.len() > 10 {
        paste_ids.pop_front();
    }

    let paste_ids = serde_json::to_string(&paste_ids).unwrap();
    cookies.add(Cookie::new("_pb_session", paste_ids));
}
