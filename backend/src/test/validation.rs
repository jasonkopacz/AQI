use super::super::*;

#[test]
fn geo_validation_rejects_out_of_bounds() {
    let invalid_lat = GeoQuery {
        lat: 91.0,
        lng: 10.0,
    };
    let invalid_lng = GeoQuery {
        lat: 10.0,
        lng: -181.0,
    };
    assert!(validate_geo_query(&invalid_lat).is_err());
    assert!(validate_geo_query(&invalid_lng).is_err());
}

#[test]
fn search_validation_rejects_empty_and_long_queries() {
    let empty = SearchQuery {
        q: "   ".to_string(),
    };
    let long = SearchQuery { q: "a".repeat(129) };
    assert!(validate_search_query(&empty).is_err());
    assert!(validate_search_query(&long).is_err());
}

#[test]
fn waqi_status_mapping_handles_common_cases() {
    let ok = serde_json::json!({ "status": "ok", "data": {} });
    let not_found = serde_json::json!({ "status": "error", "data": "Unknown station" });
    let bad_geo = serde_json::json!({ "status": "error", "data": "Invalid geo location" });
    assert_eq!(map_waqi_status(&ok), StatusCode::OK);
    assert_eq!(map_waqi_status(&not_found), StatusCode::NOT_FOUND);
    assert_eq!(map_waqi_status(&bad_geo), StatusCode::BAD_REQUEST);
}
