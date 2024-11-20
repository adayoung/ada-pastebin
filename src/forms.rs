use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct PasteForm {
    pub csrf_token: String,
    pub token: String,
}
