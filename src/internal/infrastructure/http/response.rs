use axum::{response::IntoResponse, Json};
use serde::Serialize;

/// JSend response format - success response with data
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

/// JSend error details
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

/// Creates a success response with data
pub fn success<T: Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse {
        status: "success".to_string(),
        data: Some(data),
        error: None,
    }
}

/// Creates a success response with no content
pub fn success_empty() -> ApiResponse<()> {
    ApiResponse {
        status: "success".to_string(),
        data: None,
        error: None,
    }
}

/// Creates an error response
pub fn error(code: &str, message: &str) -> ApiResponse<()> {
    ApiResponse {
        status: "error".to_string(),
        data: None,
        error: Some(ApiError {
            code: code.to_string(),
            message: message.to_string(),
        }),
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
