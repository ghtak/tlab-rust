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
    pub fn new(status_code: axum::http::StatusCode, data: Option<T>) -> Self {
        Self {
            status_code,
            data,
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

impl IntoResponse for AppResponse<()> {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, axum::Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::json;

    use super::AppResponse;

    #[test]
    fn serializes_success_data_without_status_or_error() {
        let response = AppResponse::new(StatusCode::OK, Some(json!({ "id": "order-1" })));

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({ "data": { "id": "order-1" } })
        );
    }

    #[test]
    fn serializes_error_without_status_or_data() {
        let response = AppResponse::<()>::error(StatusCode::NOT_FOUND, "Order not found".into());

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({ "error": "Order not found" } )
        );
    }
}
