use axum::Json;
use serde::{Deserialize, Serialize};

use crate::ApiError;

/// Max accepted message length (keeps the starter abuse-resistant).
pub const MAX_MESSAGE_LEN: usize = 280;

#[derive(Debug, Deserialize)]
pub struct EchoBody {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EchoReply {
    pub message: String,
}

/// `POST /api/echo` — validates `{ "message" }` and reflects it back.
/// Empty and over-long payloads fail with `422`.
pub async fn echo(Json(body): Json<EchoBody>) -> Result<Json<EchoReply>, ApiError> {
    let message = body.message.trim().to_string();
    if message.is_empty() {
        return Err(ApiError::unprocessable("message must not be empty"));
    }
    if message.chars().count() > MAX_MESSAGE_LEN {
        return Err(ApiError::unprocessable(format!(
            "message must be at most {MAX_MESSAGE_LEN} characters"
        )));
    }
    Ok(Json(EchoReply { message }))
}
