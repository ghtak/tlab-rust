use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppResponse<T> {
    #[serde(skip)]
    pub status_code: axum::http::StatusCode,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AppResponse<()> {
    pub fn ok() -> Self {
        Self {
            status_code: axum::http::StatusCode::OK,
            data: None,
            error: None,
        }
    }
}

impl<T> AppResponse<T> {
    pub fn new(status_code: axum::http::StatusCode, data: T) -> Self {
        Self {
            status_code: status_code,
            data: Some(data),
            error: None,
        }
    }

    pub fn data(data: T) -> Self {
        Self::new(axum::http::StatusCode::OK, data)
    }

    pub fn created(data: T) -> Self {
        Self::new(axum::http::StatusCode::CREATED, data)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::CONFLICT, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::error(axum::http::StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    fn error(status_code: axum::http::StatusCode, message: impl Into<String>) -> Self {
        Self {
            status_code,
            data: None,
            error: Some(message.into()),
        }
    }
}

impl<T: Serialize> IntoResponse for AppResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, axum::Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use serde_json::json;

    use super::AppResponse;

    async fn assert_response<T: serde::Serialize>(
        response: AppResponse<T>,
        expected_status: StatusCode,
        expected_body: serde_json::Value,
    ) {
        let response = response.into_response();

        assert_eq!(response.status(), expected_status);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            expected_body
        );
    }

    #[tokio::test]
    async fn responds_with_ok() {
        assert_response(AppResponse::ok(), StatusCode::OK, json!({})).await;
    }

    #[tokio::test]
    async fn responds_with_success_data() {
        assert_response(
            AppResponse::data(json!({ "id": "order-1" })),
            StatusCode::OK,
            json!({ "data": { "id": "order-1" } }),
        )
        .await;
    }

    #[tokio::test]
    async fn responds_with_created_data() {
        assert_response(
            AppResponse::created(json!({ "id": "order-1" })),
            StatusCode::CREATED,
            json!({ "data": { "id": "order-1" } }),
        )
        .await;
    }

    #[tokio::test]
    async fn responds_with_not_found_error() {
        assert_response(
            AppResponse::<()>::not_found("Order not found"),
            StatusCode::NOT_FOUND,
            json!({ "error": "Order not found" }),
        )
        .await;
    }
}
