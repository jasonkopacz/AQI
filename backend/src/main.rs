use axum::{
    extract::{Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::info;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    api_token: String,
}

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GeoQuery {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Proxy: GET /api/aqi/geo?lat=&lng=
/// Fetches real-time AQI from the WAQI geo feed endpoint.
async fn get_aqi_by_geo(
    Query(params): Query<GeoQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // WAQI uses path-style URLs: /feed/geo:{lat};{lng}/?token=...
    let url = format!(
        "https://api.waqi.info/feed/geo:{};{}/?token={}",
        params.lat, params.lng, state.api_token
    );

    match state.client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => (StatusCode::OK, Json(body)).into_response(),
            Err(e) => {
                tracing::error!("Failed to decode WAQI geo response: {e}");
                (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
            }
        },
        Err(e) => {
            tracing::error!("WAQI geo request failed: {e}");
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

/// Proxy: GET /api/aqi/search?q=
/// Returns a list of matching monitoring stations with their current AQI.
async fn search_aqi(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .client
        .get("https://api.waqi.info/search/")
        .query(&[("token", &state.api_token), ("keyword", &params.q)])
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => (StatusCode::OK, Json(body)).into_response(),
            Err(e) => {
                tracing::error!("Failed to decode WAQI search response: {e}");
                (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
            }
        },
        Err(e) => {
            tracing::error!("WAQI search request failed: {e}");
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("backend=debug".parse().unwrap()),
        )
        .init();

    let api_token = std::env::var("WAQI_API_TOKEN").unwrap_or_else(|_| {
        tracing::warn!("WAQI_API_TOKEN not set — using 'demo' token (very limited)");
        "demo".to_string()
    });

    let state = AppState {
        client: reqwest::Client::new(),
        api_token,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET]);

    // In production the frontend is built with `trunk build` and the static
    // files are served from ../frontend/dist.  During development Trunk's
    // dev-server handles the frontend and proxies /api/* here.
    let app = Router::new()
        .route("/api/aqi/geo", get(get_aqi_by_geo))
        .route("/api/aqi/search", get(search_aqi))
        .with_state(state)
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(cors);

    let addr = "0.0.0.0:3000";
    info!("Backend listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
