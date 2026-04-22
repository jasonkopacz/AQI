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
    pub no2:  Option<Measurement>,
    pub o3:   Option<Measurement>,
    pub co:   Option<Measurement>,
    pub so2:  Option<Measurement>,
    /// Temperature (°C)
    pub t:    Option<Measurement>,
    /// Relative humidity (%)
    pub h:    Option<Measurement>,
    /// Atmospheric pressure (hPa)
    pub p:    Option<Measurement>,
    /// Wind speed
    pub w:    Option<Measurement>,
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
            Self::Good            => "Good",
            Self::Moderate        => "Moderate",
            Self::SensitiveGroups => "Unhealthy for Sensitive Groups",
            Self::Unhealthy       => "Unhealthy",
            Self::VeryUnhealthy   => "Very Unhealthy",
            Self::Hazardous       => "Hazardous",
            Self::Unknown         => "No Data",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Good            => "aqi-good",
            Self::Moderate        => "aqi-moderate",
            Self::SensitiveGroups => "aqi-sensitive",
            Self::Unhealthy       => "aqi-unhealthy",
            Self::VeryUnhealthy   => "aqi-very-unhealthy",
            Self::Hazardous       => "aqi-hazardous",
            Self::Unknown         => "aqi-unknown",
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
}

impl From<u32> for AqiCategory {
    fn from(aqi: u32) -> Self {
        match aqi {
            0..=50    => Self::Good,
            51..=100  => Self::Moderate,
            101..=150 => Self::SensitiveGroups,
            151..=200 => Self::Unhealthy,
            201..=300 => Self::VeryUnhealthy,
            _         => Self::Hazardous,
        }
    }
}

// ---------------------------------------------------------------------------
// Fetch helpers  (called from Leptos components via spawn_local)
// ---------------------------------------------------------------------------

pub async fn fetch_aqi_by_geo(lat: f64, lng: f64) -> Result<AqiData, String> {
    let url = format!("/api/aqi/geo?lat={lat}&lng={lng}");

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
    let url = format!("/api/aqi/search?q={encoded}");

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
