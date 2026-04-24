use crate::build_share_url;
use crate::components::aqi_card::pct;
use crate::parse_query_coords;

// ---------------------------------------------------------------------------
// pct() — mailto: percent-encoding
// ---------------------------------------------------------------------------

#[test]
fn pct_encodes_special_url_chars() {
    assert_eq!(pct("Air & Quality? #1"), "Air%20%26%20Quality%3F%20%231");
}

#[test]
fn pct_non_ascii_city_names_pass_through() {
    assert_eq!(pct("São Paulo"), "São%20Paulo");
}

// ---------------------------------------------------------------------------
// build_share_url() — shareable URL construction
// ---------------------------------------------------------------------------

#[test]
fn build_share_url_format() {
    assert_eq!(
        build_share_url(51.5074, -0.1278, "London"),
        "?lat=51.5074&lng=-0.1278&city=London"
    );
}

#[test]
fn build_share_url_encodes_city_spaces() {
    let url = build_share_url(40.7128, -74.0060, "New York");
    assert!(url.contains("city=New%20York"), "got: {url}");
}

// ---------------------------------------------------------------------------
// parse_query_coords() — shared-link URL parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_query_coords_happy_path() {
    assert_eq!(
        parse_query_coords("lat=51.5074&lng=-0.1278&city=London"),
        Some((51.5074, -0.1278))
    );
}

#[test]
fn parse_query_coords_missing_param_returns_none() {
    assert_eq!(parse_query_coords("lat=51.5074"), None);
}

#[test]
fn parse_query_coords_invalid_value_returns_none() {
    assert_eq!(parse_query_coords("lat=abc&lng=-0.1278"), None);
}

#[test]
fn parse_query_coords_empty_returns_none() {
    assert_eq!(parse_query_coords(""), None);
}

// ---------------------------------------------------------------------------
// Round-trip: build → parse
// ---------------------------------------------------------------------------

#[test]
fn share_url_round_trips() {
    let url = build_share_url(48.8566, 2.3522, "Paris");
    let (lat, lng) = parse_query_coords(&url).expect("should parse");
    assert!((lat - 48.8566).abs() < 0.00005);
    assert!((lng - 2.3522).abs() < 0.00005);
}
