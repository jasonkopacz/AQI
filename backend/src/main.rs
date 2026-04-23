use axum::{
    extract::{Query, State},
    http::{header::HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::{io, net::SocketAddr, time::Duration};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    api_token: String,
    geo_base_url: String,
    search_base_url: String,
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

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl ApiError {
    fn bad_request(code: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
        }
    }

    fn upstream_failure() -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code: "upstream_unavailable",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(json!({
                "status": "error",
                "error": self.code
            })),
        )
            .into_response()
    }
}

fn validate_geo_query(params: &GeoQuery) -> Result<(), ApiError> {
    if !(-90.0..=90.0).contains(&params.lat) {
        return Err(ApiError::bad_request("invalid_latitude"));
    }
    if !(-180.0..=180.0).contains(&params.lng) {
        return Err(ApiError::bad_request("invalid_longitude"));
    }
    Ok(())
}

fn validate_search_query(params: &SearchQuery) -> Result<String, ApiError> {
    let cleaned = params.q.trim();
    if cleaned.is_empty() {
        return Err(ApiError::bad_request("empty_query"));
    }
    if cleaned.len() > 128 {
        return Err(ApiError::bad_request("query_too_long"));
    }
    Ok(cleaned.to_string())
}

fn map_waqi_status(body: &serde_json::Value) -> StatusCode {
    match body.get("status").and_then(|s| s.as_str()) {
        Some("ok") => StatusCode::OK,
        Some("error") => {
            let message = body
                .get("data")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_lowercase();
            if message.contains("invalid") && message.contains("geo") {
                StatusCode::BAD_REQUEST
            } else if message.contains("unknown station") || message.contains("not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("limit")
                || message.contains("quota")
                || message.contains("too many")
            {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::BAD_GATEWAY
            }
        }
        _ => StatusCode::BAD_GATEWAY,
    }
}

fn build_cors() -> CorsLayer {
    let allowed = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:8080,http://127.0.0.1:8080".to_string());
    if allowed.trim() == "*" {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET]);
    }
    let origins: Vec<HeaderValue> = allowed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();
    if origins.is_empty() {
        tracing::warn!("No valid CORS_ALLOWED_ORIGINS configured; defaulting to localhost");
        return CorsLayer::new()
            .allow_origin(AllowOrigin::list([
                HeaderValue::from_static("http://localhost:8080"),
                HeaderValue::from_static("http://127.0.0.1:8080"),
            ]))
            .allow_methods([Method::GET]);
    }
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET])
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Proxy: GET /api/aqi/geo?lat=&lng=
/// Fetches real-time AQI from the WAQI geo feed endpoint.
async fn get_aqi_by_geo(
    Query(params): Query<GeoQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    validate_geo_query(&params)?;
    // WAQI uses path-style URLs: /feed/geo:{lat};{lng}/?token=...
    let url = format!(
        "{}{};{}/?token={}",
        state.geo_base_url, params.lat, params.lng, state.api_token
    );

    match state.client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                let status = map_waqi_status(&body);
                Ok((status, Json(body)).into_response())
            }
            Err(e) => {
                tracing::error!("Failed to decode WAQI geo response: {e}");
                Err(ApiError::upstream_failure())
            }
        },
        Err(e) => {
            tracing::error!("WAQI geo request failed: {e}");
            Err(ApiError::upstream_failure())
        }
    }
}

/// Proxy: GET /api/aqi/search?q=
/// Returns a list of matching monitoring stations with their current AQI.
async fn search_aqi(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let query = validate_search_query(&params)?;
    match state
        .client
        .get(&state.search_base_url)
        .query(&[("token", &state.api_token), ("keyword", &query)])
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                let status = map_waqi_status(&body);
                Ok((status, Json(body)).into_response())
            }
            Err(e) => {
                tracing::error!("Failed to decode WAQI search response: {e}");
                Err(ApiError::upstream_failure())
            }
        },
        Err(e) => {
            tracing::error!("WAQI search request failed: {e}");
            Err(ApiError::upstream_failure())
        }
    }
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let env_filter =
        tracing_subscriber::EnvFilter::from_default_env().add_directive("backend=debug".parse()?);
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let api_token = match std::env::var("WAQI_API_TOKEN") {
        Ok(token) => token,
        Err(_) if app_env == "production" => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "WAQI_API_TOKEN must be set in production",
            )
            .into())
        }
        Err(_) => {
            tracing::warn!("WAQI_API_TOKEN not set — using 'demo' token (very limited)");
            "demo".to_string()
        }
    };

    let state = AppState {
        client: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()?,
        api_token,
        geo_base_url: "https://api.waqi.info/feed/geo:".to_string(),
        search_base_url: "https://api.waqi.info/search/".to_string(),
    };

    let cors = build_cors();

    // In production the frontend is built with `trunk build` and the static
    // files are served from ../frontend/dist.  During development Trunk's
    // dev-server handles the frontend and proxies /api/* here.
    let app = Router::new()
        .route("/api/aqi/geo", get(get_aqi_by_geo))
        .route("/api/aqi/search", get(search_aqi))
        .route("/healthz", get(healthz))
        .with_state(state)
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    info!("Backend listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod testing;
