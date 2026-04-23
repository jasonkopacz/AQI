use super::super::*;
use axum::extract::OriginalUri;
use axum::http::Request;
use axum::{body::Body, response::Response, routing::get, Json, Router};
use serde_json::Value;
use tokio::net::TcpListener;
use tower::util::ServiceExt;

async fn response_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}

fn test_state(base_url: &str) -> AppState {
    AppState {
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .expect("test reqwest client should build"),
        api_token: "test-token".to_string(),
        geo_base_url: format!("{base_url}/feed/geo:"),
        search_base_url: format!("{base_url}/search/"),
    }
}

async fn spawn_mock_upstream() -> String {
    async fn mock_fallback(OriginalUri(uri): OriginalUri) -> (StatusCode, Json<Value>) {
        if uri.path().starts_with("/feed/geo:") {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "data": {
                        "aqi": 55,
                        "city": { "name": "Mock City", "geo": [10.0, 20.0] },
                        "iaqi": {},
                        "time": { "s": "2026-04-23 10:00:00", "iso": null }
                    }
                })),
            );
        }
        if uri.path().starts_with("/search/") {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "error",
                    "data": "Unknown station"
                })),
            );
        }
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "status": "error",
                "data": "Unhandled mock route"
            })),
        )
    }

    let app = Router::new().fallback(get(mock_fallback));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let addr = listener.local_addr().expect("mock local addr");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock server should run");
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn geo_endpoint_rejects_invalid_query() {
    let app = Router::new()
        .route("/api/aqi/geo", get(get_aqi_by_geo))
        .with_state(test_state("http://127.0.0.1:1"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/aqi/geo?lat=95&lng=10")
                .method("GET")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"], "invalid_latitude");
}

#[tokio::test]
async fn geo_endpoint_returns_ok_from_upstream() {
    let upstream = spawn_mock_upstream().await;
    let app = Router::new()
        .route("/api/aqi/geo", get(get_aqi_by_geo))
        .with_state(test_state(&upstream));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/aqi/geo?lat=10&lng=20")
                .method("GET")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["data"]["city"]["name"], "Mock City");
}

#[tokio::test]
async fn search_endpoint_maps_upstream_error_status() {
    let upstream = spawn_mock_upstream().await;
    let app = Router::new()
        .route("/api/aqi/search", get(search_aqi))
        .with_state(test_state(&upstream));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/aqi/search?q=missing-station")
                .method("GET")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["status"], "error");
    assert_eq!(body["data"], "Unknown station");
}

#[tokio::test]
async fn search_endpoint_rejects_empty_query() {
    let app = Router::new()
        .route("/api/aqi/search", get(search_aqi))
        .with_state(test_state("http://127.0.0.1:1"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/aqi/search?q=%20%20")
                .method("GET")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"], "empty_query");
}
