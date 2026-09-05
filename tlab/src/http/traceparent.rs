use tracing::info_span;
use tracing_subscriber::registry::LookupSpan;

const TRACEPARENT_VERSION: &str = "00";
const ROOT_PARENT_SPAN_ID: &str = "0000000000000000";

/// Parses a version 00 W3C `traceparent` header as `(trace_id, parent_span_id)`.
fn parse_traceparent(value: &str) -> Option<(String, String)> {
    let mut parts = value.split('-');
    let (version, trace_id, parent_span_id, flags) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);

    (parts.next().is_none()
        && version == TRACEPARENT_VERSION
        && is_valid_id(trace_id, 32)
        && is_valid_id(parent_span_id, 16)
        && is_hex(flags, 2))
    .then(|| (trace_id.to_owned(), parent_span_id.to_owned()))
}

fn is_valid_id(value: &str, length: usize) -> bool {
    is_hex(value, length) && value.bytes().any(|byte| byte != b'0')
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn new_span_id() -> String {
    loop {
        let span_id = rand::random::<u64>();
        if span_id != 0 {
            return format!("{span_id:016x}");
        }
    }
}

pub fn new_http_request_span<B>(req: &axum::http::Request<B>) -> tracing::Span {
    let headers = req.headers();

    // Header에서 traceparent 추출
    let (trace_id, parent_span_id) = headers
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_traceparent)
        .unwrap_or_else(|| {
            // 헤더가 없으면 새로 생성 (최초 진입점)
            let new_trace_id = uuid::Uuid::new_v4().simple().to_string(); // 32자리 hex
            (new_trace_id, ROOT_PARENT_SPAN_ID.to_owned())
        });

    // 수신 시점에 "나의 새로운 Span ID" 생성 (16자리 hex)
    let span_id = new_span_id();

    // tracing::Span 생성 및 필드 바인딩
    let span = info_span!(
        "http_request",
        method = %req.method(),
        uri = %req.uri(),
        trace_id = %trace_id,
        span_id = %span_id,
        parent_span_id = %parent_span_id
    );

    span.with_subscriber(|(id, subscriber)| {
        if let Some(registry) = subscriber.downcast_ref::<tracing_subscriber::Registry>() {
            if let Some(span_ref) = registry.span(id) {
                let mut extensions = span_ref.extensions_mut();

                // 나만의 컨텍스트 구조체를 만들어 주입 [1]
                extensions.insert(TraceparentData {
                    trace_id: trace_id.clone(),
                    span_id: span_id.clone(),
                });
            }
        }
    });
    span
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceparentData {
    pub trace_id: String,
    pub span_id: String,
}

pub fn get_current_traceparent() -> Option<TraceparentData> {
    tracing::Span::current()
        .with_subscriber(|(id, subscriber)| {
            subscriber
                .downcast_ref::<tracing_subscriber::Registry>()?
                .span(id)?
                .extensions()
                .get::<TraceparentData>()
                .cloned()
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;
    use tower_http::trace::TraceLayer;
    use tracing_subscriber::fmt::format::FmtSpan;

    #[test]
    fn test_parse_traceparent() {
        let header_val = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            parse_traceparent(header_val),
            Some((
                "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
                "00f067aa0ba902b7".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_traceparent_rejects_invalid_values() {
        for value in [
            "invalid-header",
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-zz",
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        ] {
            assert_eq!(parse_traceparent(value), None, "{value}");
        }
    }

    #[test]
    fn test_new_span() {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_test_writer()
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let app = axum::Router::new()
                        .route(
                            "/orders",
                            get(|| async {
                                let data = get_current_traceparent();
                                assert!(data.is_some());
                                let data = data.unwrap();
                                assert_eq!(data.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
                                assert!(is_valid_id(&data.span_id, 16));
                                tracing::info!(
                                    "Current Traceparent: trace_id={}, span_id={}",
                                    data.trace_id,
                                    data.span_id
                                );
                                "Hello, world!"
                            }),
                        )
                        .layer(TraceLayer::new_for_http().make_span_with(new_http_request_span));

                    let response = app
                        .oneshot(
                            Request::builder()
                                .uri("/orders")
                                .header(
                                    "traceparent",
                                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                                )
                                .body(Body::empty())
                                .unwrap(),
                        )
                        .await
                        .unwrap();

                    assert_eq!(response.status(), StatusCode::OK);
                });
        });
    }
}
