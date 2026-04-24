use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// WAQI geo-feed response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeoResponse {
    pub status: String,
    pub data: Option<AqiData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AqiData {
    /// The composite AQI number (US EPA scale).
    pub aqi: serde_json::Value, // can be int or "-"
    pub city: CityInfo,
    #[serde(rename = "dominentpol")]
    pub dominant_pol: Option<String>,
    pub iaqi: Iaqi,
    pub time: TimeInfo,
    /// Optional multi-day forecast block provided by WAQI.
    pub forecast: Option<ForecastData>,
}

// ---------------------------------------------------------------------------
// Forecast types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ForecastData {
    pub daily: Option<DailyForecast>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DailyForecast {
    pub pm25: Option<Vec<DailyEntry>>,
    pub pm10: Option<Vec<DailyEntry>>,
    pub o3: Option<Vec<DailyEntry>>,
    pub uvi: Option<Vec<DailyEntry>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DailyEntry {
    /// Calendar date string, e.g. "2026-04-23"
    pub day: String,
    pub avg: Option<i32>,
    pub max: Option<i32>,
    pub min: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ForecastDayDetails {
    pub day: String,
    pub primary: DailyEntry,
    pub pm25: Option<DailyEntry>,
    pub pm10: Option<DailyEntry>,
    pub o3: Option<DailyEntry>,
    pub uvi: Option<DailyEntry>,
}

impl AqiData {
    /// Returns the numeric AQI, or None if it is a placeholder like "-".
    pub fn aqi_number(&self) -> Option<u32> {
        match &self.aqi {
            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u32),
            serde_json::Value::String(s) => s.parse::<u32>().ok(),
            _ => None,
        }
    }

    pub fn category(&self) -> AqiCategory {
        match self.aqi_number() {
            Some(n) => AqiCategory::from(n),
            None => AqiCategory::Unknown,
        }
    }

    /// Returns today's UV index average from forecast data, if available.
    pub fn uvi_today(&self) -> Option<f64> {
        let daily = self.forecast.as_ref()?.daily.as_ref()?;
        let today = self.time.s.as_deref().map(|s| &s[..s.len().min(10)])?;
        daily
            .uvi
            .as_ref()?
            .iter()
            .find(|e| e.day == today)
            .and_then(|e| e.avg.map(|v| v as f64))
    }

    /// Returns daily avg AQI entries to drive the sparkline.
    /// Prefers pm25 (most relevant), falls back to pm10 then o3.
    /// Returns an empty vec if no forecast data is present.
    pub fn sparkline_data(&self) -> Vec<DailyEntry> {
        let daily = match self.forecast.as_ref().and_then(|f| f.daily.as_ref()) {
            Some(d) => d,
            None => return vec![],
        };
        let series = daily
            .pm25
            .as_ref()
            .filter(|v| !v.is_empty())
            .or_else(|| daily.pm10.as_ref().filter(|v| !v.is_empty()))
            .or_else(|| daily.o3.as_ref().filter(|v| !v.is_empty()));

        series.cloned().unwrap_or_default()
    }

    /// Returns per-day forecast details for the daily cards.
    /// The card's primary AQI values follow the same fallback as sparkline_data:
    /// pm25 -> pm10 -> o3.
    pub fn forecast_day_details(&self) -> Vec<ForecastDayDetails> {
        let daily = match self.forecast.as_ref().and_then(|f| f.daily.as_ref()) {
            Some(d) => d,
            None => return vec![],
        };

        let primary = daily
            .pm25
            .as_ref()
            .filter(|v| !v.is_empty())
            .or_else(|| daily.pm10.as_ref().filter(|v| !v.is_empty()))
            .or_else(|| daily.o3.as_ref().filter(|v| !v.is_empty()));

        let Some(primary_series) = primary else {
            return vec![];
        };

        primary_series
            .iter()
            .cloned()
            .map(|entry| {
                let day = entry.day.clone();
                let pm25 = daily
                    .pm25
                    .as_ref()
                    .and_then(|items| items.iter().find(|e| e.day == day).cloned());
                let pm10 = daily
                    .pm10
                    .as_ref()
                    .and_then(|items| items.iter().find(|e| e.day == day).cloned());
                let o3 = daily
                    .o3
                    .as_ref()
                    .and_then(|items| items.iter().find(|e| e.day == day).cloned());
                let uvi = daily
                    .uvi
                    .as_ref()
                    .and_then(|items| items.iter().find(|e| e.day == day).cloned());

                ForecastDayDetails {
                    day,
                    primary: entry,
                    pm25,
                    pm10,
                    o3,
                    uvi,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CityInfo {
    pub name: String,
    pub geo: Vec<f64>,
}

/// Individual Air Quality Index sub-components.
/// All fields optional — not every station reports every pollutant.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Iaqi {
    pub pm25: Option<Measurement>,
    pub pm10: Option<Measurement>,
    pub no2: Option<Measurement>,
    pub o3: Option<Measurement>,
    pub co: Option<Measurement>,
    pub so2: Option<Measurement>,
    /// Temperature (°C)
    pub t: Option<Measurement>,
    /// Relative humidity (%)
    pub h: Option<Measurement>,
    /// Atmospheric pressure (hPa)
    pub p: Option<Measurement>,
    /// Wind speed
    pub w: Option<Measurement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Measurement {
    pub v: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeInfo {
    /// Human-readable timestamp in the station's local timezone.
    pub s: Option<String>,
    pub iso: Option<String>,
}

// ---------------------------------------------------------------------------
// WAQI search response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub status: String,
    pub data: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult {
    pub uid: i64,
    pub aqi: String,
    pub station: Station,
}

impl SearchResult {
    pub fn aqi_number(&self) -> Option<u32> {
        self.aqi.trim().parse::<u32>().ok()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Station {
    pub name: String,
    pub geo: Vec<f64>,
    pub country: Option<String>,
}

// ---------------------------------------------------------------------------
// AQI category
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AqiCategory {
    Good,
    Moderate,
    SensitiveGroups,
    Unhealthy,
    VeryUnhealthy,
    Hazardous,
    Unknown,
}

impl AqiCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Good => "Good",
            Self::Moderate => "Moderate",
            Self::SensitiveGroups => "Unhealthy for Sensitive Groups",
            Self::Unhealthy => "Unhealthy",
            Self::VeryUnhealthy => "Very Unhealthy",
            Self::Hazardous => "Hazardous",
            Self::Unknown => "No Data",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Good => "aqi-good",
            Self::Moderate => "aqi-moderate",
            Self::SensitiveGroups => "aqi-sensitive",
            Self::Unhealthy => "aqi-unhealthy",
            Self::VeryUnhealthy => "aqi-very-unhealthy",
            Self::Hazardous => "aqi-hazardous",
            Self::Unknown => "aqi-unknown",
        }
    }

    pub fn advice(&self) -> &'static str {
        match self {
            Self::Good            => "Air quality is satisfactory. Go outside and enjoy!",
            Self::Moderate        => "Acceptable air quality. Unusually sensitive people should consider limiting prolonged outdoor exertion.",
            Self::SensitiveGroups => "Members of sensitive groups may experience health effects. General public is less likely to be affected.",
            Self::Unhealthy       => "Everyone may begin to experience health effects. Sensitive groups may experience more serious effects.",
            Self::VeryUnhealthy   => "Health alert: everyone may experience more serious health effects.",
            Self::Hazardous       => "Health warning of emergency conditions. The entire population is more likely to be affected.",
            Self::Unknown         => "Data unavailable for this location.",
        }
    }

    /// Emoji icon representing the health risk level.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Good => "✅",
            Self::Moderate => "🟡",
            Self::SensitiveGroups => "⚠️",
            Self::Unhealthy => "🔴",
            Self::VeryUnhealthy => "🟣",
            Self::Hazardous => "☠️",
            Self::Unknown => "❓",
        }
    }

    /// Groups who should take extra precaution at this AQI level.
    /// Returns an empty slice when no specific groups are at elevated risk.
    pub fn sensitive_groups(&self) -> &'static [&'static str] {
        match self {
            Self::Good => &[],
            Self::Moderate => &["People unusually sensitive to air pollution"],
            Self::SensitiveGroups => &[
                "Children",
                "Elderly",
                "Asthma / lung disease",
                "Heart disease",
            ],
            Self::Unhealthy => &[
                "Children",
                "Elderly",
                "Asthma / lung disease",
                "Heart disease",
                "Everyone",
            ],
            Self::VeryUnhealthy => &["Everyone"],
            Self::Hazardous => &["Everyone — emergency conditions"],
            Self::Unknown => &[],
        }
    }

    /// Specific, actionable recommendations for this AQI level.
    pub fn recommendations(&self) -> &'static [&'static str] {
        match self {
            Self::Good => &[
                "Enjoy outdoor activities freely",
                "Great day for exercise outside",
                "Windows can be left open",
            ],
            Self::Moderate => &[
                "Sensitive individuals should limit prolonged outdoor exertion",
                "Consider moving intense workouts indoors if you feel symptoms",
                "Children with asthma should watch for symptoms",
            ],
            Self::SensitiveGroups => &[
                "Sensitive groups should reduce prolonged outdoor exertion",
                "Take more breaks during outdoor activities",
                "Keep asthma medication handy",
                "Watch children for symptoms like coughing or shortness of breath",
            ],
            Self::Unhealthy => &[
                "Everyone should reduce prolonged outdoor exertion",
                "Sensitive groups should avoid outdoor activity",
                "Move exercise indoors",
                "Keep windows closed",
                "Consider wearing an N95 mask outdoors",
            ],
            Self::VeryUnhealthy => &[
                "Everyone should avoid outdoor exertion",
                "Sensitive groups should remain indoors",
                "Run air purifiers if available",
                "Keep all windows and doors closed",
                "Wear an N95 mask if you must go outside",
            ],
            Self::Hazardous => &[
                "Stay indoors — emergency conditions",
                "Keep windows and doors sealed",
                "Run air purifiers on highest setting",
                "Avoid ALL outdoor activity",
                "Wear N95 mask if going outside is unavoidable",
                "Seek medical attention if experiencing symptoms",
            ],
            Self::Unknown => &["Check back when data becomes available"],
        }
    }
}

impl From<u32> for AqiCategory {
    fn from(aqi: u32) -> Self {
        match aqi {
            0..=50 => Self::Good,
            51..=100 => Self::Moderate,
            101..=150 => Self::SensitiveGroups,
            151..=200 => Self::Unhealthy,
            201..=300 => Self::VeryUnhealthy,
            _ => Self::Hazardous,
        }
    }
}

// ---------------------------------------------------------------------------
// Fetch helpers  (called from Leptos components via spawn_local)
// ---------------------------------------------------------------------------

// When WAQI_API_TOKEN is set at build time (e.g. in the GitHub Pages CI job)
// the frontend calls WAQI directly, bypassing the backend proxy.
// In local development the token is absent and calls go to /api/... which
// Trunk proxies to the Axum backend.
const WAQI_TOKEN: Option<&str> = option_env!("WAQI_API_TOKEN");

pub async fn fetch_aqi_by_geo(lat: f64, lng: f64) -> Result<AqiData, String> {
    let url = match WAQI_TOKEN {
        Some(token) => format!("https://api.waqi.info/feed/geo:{lat};{lng}/?token={token}"),
        None => format!("/api/aqi/geo?lat={lat}&lng={lng}"),
    };

    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.ok() {
        return Err(format!("Server returned status {}", resp.status()));
    }

    let geo: GeoResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    if geo.status != "ok" {
        return Err(format!("WAQI API error (status: {})", geo.status));
    }

    geo.data.ok_or_else(|| "No data in response".to_string())
}

pub async fn fetch_aqi_search(query: &str) -> Result<Vec<SearchResult>, String> {
    let encoded = js_sys::encode_uri_component(query);
    let url = match WAQI_TOKEN {
        Some(token) => format!("https://api.waqi.info/search/?token={token}&keyword={encoded}"),
        None => format!("/api/aqi/search?q={encoded}"),
    };

    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.ok() {
        return Err(format!("Server returned status {}", resp.status()));
    }

    let search: SearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    if search.status != "ok" {
        return Err(format!("WAQI API error (status: {})", search.status));
    }

    Ok(search.data)
}
