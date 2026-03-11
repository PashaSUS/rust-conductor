use serde_json;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Redis error: {0}")]
    Redis(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Serialization error: {0}")]
    Serde(String),
}

impl actix_web::ResponseError for EngineError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            EngineError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            EngineError::InvalidState(_) => actix_web::http::StatusCode::CONFLICT,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        let body = serde_json::json!({
            "status": self.status_code().as_u16(),
            "message": self.to_string(),
        });
        actix_web::HttpResponse::build(self.status_code()).json(body)
    }
}
