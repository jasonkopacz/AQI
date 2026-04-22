use leptos::prelude::*;
use crate::api::AqiData;

#[component]
pub fn AqiCard(data: AqiData) -> impl IntoView {
    let aqi_num = data.aqi_number();
    let category = data.category();
    let css_class = category.css_class().to_string();
    let label = category.label().to_string();
    let advice = category.advice().to_string();

    // Format the timestamp
    let timestamp = data
        .time
        .s
        .as_deref()
        .unwrap_or("—")
        .to_string();

    // Dominant pollutant, prettified
    let dominant = data
        .dominant_pol
        .as_deref()
        .map(prettify_pollutant)
        .unwrap_or_default()
        .to_string();

    view! {
        <div class=format!("aqi-card {css_class}")>
            <div class="aqi-card__header">
                <h2 class="aqi-card__city">{data.city.name.clone()}</h2>
                <span class="aqi-card__timestamp">"Updated: " {timestamp}</span>
            </div>

            <div class="aqi-card__body">
                <div class="aqi-gauge">
                    <span class="aqi-gauge__value">
                        {aqi_num.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string())}
                    </span>
                    <span class="aqi-gauge__label">AQI</span>
                </div>

                <div class="aqi-card__info">
                    <p class="aqi-card__category">{label}</p>
                    {(!dominant.is_empty()).then(|| view! {
                        <p class="aqi-card__dominant">
                            "Main pollutant: " <strong>{dominant}</strong>
                        </p>
                    })}
                    <p class="aqi-card__advice">{advice}</p>
                </div>
            </div>

            <AqiScale current={aqi_num} />
        </div>
    }
}

/// A horizontal scale bar illustrating where the current AQI sits.
#[component]
fn AqiScale(current: Option<u32>) -> impl IntoView {
    let segments = [
        ("Good",       "aqi-good",          50u32),
        ("Moderate",   "aqi-moderate",      100),
        ("USG",        "aqi-sensitive",     150),
        ("Unhealthy",  "aqi-unhealthy",     200),
        ("Very",       "aqi-very-unhealthy",300),
        ("Hazardous",  "aqi-hazardous",     500),
    ];

    view! {
        <div class="aqi-scale">
            {segments.iter().map(|(label, cls, max)| {
                let label = *label;
                let cls = *cls;
                let max = *max;
                let is_active = current.map(|c| {
                    let min = match max {
                        50  => 0,
                        100 => 51,
                        150 => 101,
                        200 => 151,
                        300 => 201,
                        _   => 301,
                    };
                    c >= min && c <= max
                }).unwrap_or(false);

                view! {
                    <div
                        class=format!("aqi-scale__segment {cls}{}", if is_active { " active" } else { "" })
                        title=format!("{label} (0–{max})")
                    >
                        <span class="aqi-scale__label">{label}</span>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

fn prettify_pollutant(raw: &str) -> &str {
    match raw {
        "pm25" => "PM₂.₅",
        "pm10" => "PM₁₀",
        "no2"  => "NO₂",
        "o3"   => "O₃",
        "co"   => "CO",
        "so2"  => "SO₂",
        other  => other,
    }
}
