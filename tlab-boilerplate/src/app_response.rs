use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppResponse<T: Serialize> {
    #[serde(skip)]
    pub status_code: axum::http::StatusCode,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> AppResponse<T> {
    pub fn code(status_code: axum::http::StatusCode) -> Self {
        Self {
            status_code,
            data: None,
            error: None,
        }
    }

    pub fn data(status_code: axum::http::StatusCode, data: T) -> Self {
        Self {
            status_code,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(status_code: axum::http::StatusCode, message: String) -> Self {
        Self {
            status_code,
            data: None,
            error: Some(message),
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
    async fn responds_with_empty_success_body() {
        assert_response(
            AppResponse::<()>::code(StatusCode::OK),
            StatusCode::OK,
            json!({}),
        )
        .await;
    }

    #[tokio::test]
    async fn responds_with_success_data() {
        assert_response(
            AppResponse::data(StatusCode::OK, json!({ "id": "order-1" })),
            StatusCode::OK,
            json!({ "data": { "id": "order-1" } }),
        )
        .await;
    }

    #[tokio::test]
    async fn responds_with_error() {
        assert_response(
            AppResponse::<()>::error(StatusCode::NOT_FOUND, "Order not found".into()),
            StatusCode::NOT_FOUND,
            json!({ "error": "Order not found" }),
        )
        .await;
    }
}
