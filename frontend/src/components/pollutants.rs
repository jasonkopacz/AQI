use leptos::prelude::*;
use crate::api::Iaqi;

#[component]
pub fn PollutantsGrid(iaqi: Iaqi) -> impl IntoView {
    view! {
        <section class="pollutants">
            <h3 class="pollutants__title">"Pollutants & Conditions"</h3>
            <div class="pollutants__grid">
                <PollutantTile
                    name="PM₂.₅"
                    unit="µg/m³"
                    value={iaqi.pm25.map(|m| m.v)}
                    description="Fine particulate matter"
                />
                <PollutantTile
                    name="PM₁₀"
                    unit="µg/m³"
                    value={iaqi.pm10.map(|m| m.v)}
                    description="Coarse particulate matter"
                />
                <PollutantTile
                    name="O₃"
                    unit="ppb"
                    value={iaqi.o3.map(|m| m.v)}
                    description="Ozone"
                />
                <PollutantTile
                    name="NO₂"
                    unit="ppb"
                    value={iaqi.no2.map(|m| m.v)}
                    description="Nitrogen dioxide"
                />
                <PollutantTile
                    name="SO₂"
                    unit="ppb"
                    value={iaqi.so2.map(|m| m.v)}
                    description="Sulphur dioxide"
                />
                <PollutantTile
                    name="CO"
                    unit="ppm"
                    value={iaqi.co.map(|m| m.v)}
                    description="Carbon monoxide"
                />
            </div>

            // Weather conditions if available
            {
                let has_weather = iaqi.t.is_some() || iaqi.h.is_some() || iaqi.p.is_some();
                has_weather.then(|| view! {
                    <div class="weather-row">
                        {iaqi.t.map(|m| view! {
                            <WeatherTile icon="🌡" label="Temperature" value=format!("{:.1}°C", m.v) />
                        })}
                        {iaqi.h.map(|m| view! {
                            <WeatherTile icon="💧" label="Humidity" value=format!("{:.0}%", m.v) />
                        })}
                        {iaqi.p.map(|m| view! {
                            <WeatherTile icon="🔵" label="Pressure" value=format!("{:.0} hPa", m.v) />
                        })}
                        {iaqi.w.map(|m| view! {
                            <WeatherTile icon="💨" label="Wind" value=format!("{:.1} m/s", m.v) />
                        })}
                    </div>
                })
            }
        </section>
    }
}

#[component]
fn PollutantTile(
    name: &'static str,
    unit: &'static str,
    value: Option<f64>,
    description: &'static str,
) -> impl IntoView {
    let display = value
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "—".to_string());

    view! {
        <div class="pollutant-tile">
            <span class="pollutant-tile__name">{name}</span>
            <span class="pollutant-tile__value">
                {display}
                {value.map(|_| view! {
                    <span class="pollutant-tile__unit">{unit}</span>
                })}
            </span>
            <span class="pollutant-tile__desc">{description}</span>
        </div>
    }
}

#[component]
fn WeatherTile(icon: &'static str, label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="weather-tile">
            <span class="weather-tile__icon">{icon}</span>
            <span class="weather-tile__label">{label}</span>
            <span class="weather-tile__value">{value}</span>
        </div>
    }
}
