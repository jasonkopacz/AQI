use crate::api::*;

fn make_data_with_forecast(forecast: ForecastData) -> AqiData {
    AqiData {
        aqi: serde_json::json!(42),
        city: CityInfo {
            name: "Test City".to_string(),
            geo: vec![1.0, 2.0],
        },
        dominant_pol: Some("pm25".to_string()),
        iaqi: Iaqi::default(),
        time: TimeInfo {
            s: Some("2026-04-23 10:00:00".to_string()),
            iso: None,
        },
        forecast: Some(forecast),
    }
}

#[test]
fn aqi_number_parses_number_and_string() {
    let numeric = AqiData {
        aqi: serde_json::json!(75),
        city: CityInfo {
            name: "City".to_string(),
            geo: vec![],
        },
        dominant_pol: None,
        iaqi: Iaqi::default(),
        time: TimeInfo { s: None, iso: None },
        forecast: None,
    };
    assert_eq!(numeric.aqi_number(), Some(75));
    let stringy = AqiData {
        aqi: serde_json::json!("88"),
        ..numeric.clone()
    };
    let invalid = AqiData {
        aqi: serde_json::json!("-"),
        ..numeric.clone()
    };
    assert_eq!(stringy.aqi_number(), Some(88));
    assert_eq!(invalid.aqi_number(), None);
}

#[test]
fn category_unknown_when_aqi_unparseable() {
    let data = AqiData {
        aqi: serde_json::json!("-"),
        city: CityInfo {
            name: "City".to_string(),
            geo: vec![],
        },
        dominant_pol: None,
        iaqi: Iaqi::default(),
        time: TimeInfo { s: None, iso: None },
        forecast: None,
    };
    assert_eq!(data.category(), AqiCategory::Unknown);
}

#[test]
fn sparkline_data_empty_without_forecast() {
    let data = AqiData {
        aqi: serde_json::json!(1),
        city: CityInfo {
            name: "C".to_string(),
            geo: vec![],
        },
        dominant_pol: None,
        iaqi: Iaqi::default(),
        time: TimeInfo { s: None, iso: None },
        forecast: None,
    };
    assert!(data.sparkline_data().is_empty());
}

#[test]
fn search_result_aqi_number_trims_and_rejects_invalid() {
    let ok = SearchResult {
        uid: 1,
        aqi: " 42 ".to_string(),
        station: Station {
            name: "S".to_string(),
            geo: vec![],
            country: None,
        },
    };
    let bad = SearchResult {
        aqi: "n/a".to_string(),
        ..ok.clone()
    };
    assert_eq!(ok.aqi_number(), Some(42));
    assert_eq!(bad.aqi_number(), None);
}

#[test]
fn geo_response_deserializes_minimal_ok_payload() {
    let json = r#"{"status":"ok","data":{"aqi":29,"city":{"name":"X","geo":[0.0,0.0]},"dominentpol":"pm25","iaqi":{},"time":{"s":"2026-04-23 12:00:00"}}}"#;
    let geo: GeoResponse = serde_json::from_str(json).expect("valid JSON");
    assert_eq!(geo.status, "ok");
    let data = geo.data.expect("data present");
    assert_eq!(data.aqi_number(), Some(29));
    assert_eq!(data.city.name, "X");
}

#[test]
fn category_boundaries_match_epa_ranges() {
    assert_eq!(AqiCategory::from(50), AqiCategory::Good);
    assert_eq!(AqiCategory::from(100), AqiCategory::Moderate);
    assert_eq!(AqiCategory::from(150), AqiCategory::SensitiveGroups);
    assert_eq!(AqiCategory::from(200), AqiCategory::Unhealthy);
    assert_eq!(AqiCategory::from(300), AqiCategory::VeryUnhealthy);
    assert_eq!(AqiCategory::from(301), AqiCategory::Hazardous);
}

#[test]
fn sparkline_data_prefers_pm25_then_pm10_then_o3() {
    let pm10_only = ForecastData {
        daily: Some(DailyForecast {
            pm25: None,
            pm10: Some(vec![DailyEntry {
                day: "2026-04-23".to_string(),
                avg: Some(31),
                min: Some(20),
                max: Some(45),
            }]),
            o3: Some(vec![DailyEntry {
                day: "2026-04-23".to_string(),
                avg: Some(99),
                min: Some(80),
                max: Some(110),
            }]),
            uvi: None,
        }),
    };
    let data = make_data_with_forecast(pm10_only);
    let vals: Vec<i32> = data
        .sparkline_data()
        .into_iter()
        .filter_map(|d| d.avg)
        .collect();
    assert_eq!(vals, vec![31]);
}

#[test]
fn forecast_day_details_aligns_secondary_series_by_day() {
    let forecast = ForecastData {
        daily: Some(DailyForecast {
            pm25: Some(vec![DailyEntry {
                day: "2026-04-23".to_string(),
                avg: Some(40),
                min: Some(30),
                max: Some(55),
            }]),
            pm10: Some(vec![DailyEntry {
                day: "2026-04-23".to_string(),
                avg: Some(20),
                min: Some(12),
                max: Some(28),
            }]),
            o3: None,
            uvi: Some(vec![DailyEntry {
                day: "2026-04-23".to_string(),
                avg: Some(3),
                min: Some(1),
                max: Some(5),
            }]),
        }),
    };
    let data = make_data_with_forecast(forecast);
    let details = data.forecast_day_details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].primary.avg, Some(40));
    assert_eq!(details[0].pm10.as_ref().and_then(|d| d.avg), Some(20));
    assert_eq!(details[0].uvi.as_ref().and_then(|d| d.avg), Some(3));
}

#[test]
fn uvi_today_matches_exact_day() {
    let forecast = ForecastData {
        daily: Some(DailyForecast {
            pm25: None,
            pm10: None,
            o3: None,
            uvi: Some(vec![
                DailyEntry {
                    day: "2026-04-22".to_string(),
                    avg: Some(6),
                    min: Some(4),
                    max: Some(8),
                },
                DailyEntry {
                    day: "2026-04-23".to_string(),
                    avg: Some(4),
                    min: Some(2),
                    max: Some(5),
                },
            ]),
        }),
    };
    let data = make_data_with_forecast(forecast);
    assert_eq!(data.uvi_today(), Some(4.0));
}
