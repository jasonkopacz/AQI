use leptos::prelude::*;
use crate::api::Iaqi;

#[component]
pub fn PollutantsGrid(
    iaqi: Iaqi,
    uvi: Option<f64>,
) -> impl IntoView {
    let use_fahrenheit = RwSignal::new(false);

    view! {
        <section class="pollutants">
            <div class="pollutants__header">
                <h3 class="pollutants__title">"Pollutants & Conditions"</h3>
            </div>
            <div class="pollutants__grid">
                <PollutantTile
                    name="PM₂.₅"
                    unit="µg/m³"
                    value={iaqi.pm25.map(|m| m.v)}
                    description="Fine particulate matter"
                    tooltip="Particles smaller than 2.5 micrometres. They penetrate deep into the lungs and can enter the bloodstream, causing respiratory and cardiovascular disease."
                />
                <PollutantTile
                    name="PM₁₀"
                    unit="µg/m³"
                    value={iaqi.pm10.map(|m| m.v)}
                    description="Coarse particulate matter"
                    tooltip="Particles between 2.5–10 micrometres (dust, pollen, mould). Irritate the nose, throat, and upper airways. Safe threshold: 50 µg/m³ (24-hr avg)."
                />
                <PollutantTile
                    name="O₃"
                    unit="ppb"
                    value={iaqi.o3.map(|m| m.v)}
                    description="Ozone"
                    tooltip="Ground-level ozone forms when sunlight reacts with NOₓ and VOCs from vehicles and industry. Irritates lungs; worsens asthma and bronchitis. Higher on sunny afternoons."
                />
                <PollutantTile
                    name="NO₂"
                    unit="ppb"
                    value={iaqi.no2.map(|m| m.v)}
                    description="Nitrogen dioxide"
                    tooltip="Emitted mainly by vehicle exhaust and power plants. Irritates the respiratory tract and contributes to ozone and particulate formation. Safe threshold: 53 ppb (annual avg)."
                />
                <PollutantTile
                    name="SO₂"
                    unit="ppb"
                    value={iaqi.so2.map(|m| m.v)}
                    description="Sulphur dioxide"
                    tooltip="Produced by burning coal, oil, and volcanic eruptions. Causes respiratory irritation and acid rain. At high levels, can trigger asthma attacks. Safe threshold: 75 ppb (1-hr avg)."
                />
                <PollutantTile
                    name="CO"
                    unit="ppm"
                    value={iaqi.co.map(|m| m.v)}
                    description="Carbon monoxide"
                    tooltip="Colourless, odourless gas from incomplete combustion (vehicles, fires). Reduces oxygen delivery in blood. Dangerous at high concentrations. Safe threshold: 9 ppm (8-hr avg)."
                />
            </div>

            // Weather conditions if available
            {
                let has_weather = iaqi.t.is_some() || iaqi.h.is_some() || iaqi.p.is_some() || uvi.is_some();
                has_weather.then(|| view! {
                    <div class="weather-row">
                        {iaqi.t.map(|m| {
                            let celsius = m.v;
                            view! {
                                <TempTile celsius=celsius use_fahrenheit=use_fahrenheit />
                            }
                        })}
                        {iaqi.h.map(|m| { let v = m.v; view! {
                            <WeatherTile icon="💧" label="Humidity" value=move || format!("{:.0}%", v) />
                        }})}
                        {iaqi.p.map(|m| { let v = m.v; view! {
                            <WeatherTile icon="🔵" label="Pressure" value=move || format!("{:.0} hPa", v) />
                        }})}
                        {iaqi.w.map(|m| { let v = m.v; view! {
                            <WeatherTile icon="💨" label="Wind" value=move || format!("{:.1} m/s", v) />
                        }})}
                        {uvi.map(|v| view! {
                            <WeatherTile
                                icon=uvi_icon(v)
                                label="UV Index"
                                value=move || format!("{:.0} — {}", v, uvi_label(v))
                            />
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
    #[prop(default = "")]
    tooltip: &'static str,
) -> impl IntoView {
    let display = value
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "—".to_string());

    let show_tooltip = RwSignal::new(false);

    view! {
        <div
            class="pollutant-tile"
            on:mouseenter=move |_| show_tooltip.set(true)
            on:mouseleave=move |_| show_tooltip.set(false)
            on:focus=move |_| show_tooltip.set(true)
            on:blur=move |_| show_tooltip.set(false)
            tabindex="0"
        >
            <div class="pollutant-tile__header">
                <span class="pollutant-tile__name">{name}</span>
                {(!tooltip.is_empty()).then(|| view! {
                    <span class="pollutant-tile__info-icon" aria-label="More info">"ⓘ"</span>
                })}
            </div>
            <span class="pollutant-tile__value">
                {display}
                {value.map(|_| view! {
                    <span class="pollutant-tile__unit">{unit}</span>
                })}
            </span>
            <span class="pollutant-tile__desc">{description}</span>

            // Tooltip popup
            {(!tooltip.is_empty()).then(|| view! {
                <div
                    class="pollutant-tooltip"
                    class:pollutant-tooltip--visible=move || show_tooltip.get()
                    role="tooltip"
                >
                    {tooltip}
                </div>
            })}
        </div>
    }
}

/// Temperature tile — click anywhere to toggle °C / °F.
#[component]
fn TempTile(celsius: f64, use_fahrenheit: RwSignal<bool>) -> impl IntoView {
    view! {
        <button
            class="weather-tile weather-tile--temp"
            title="Click to toggle °C / °F"
            on:click=move |_| use_fahrenheit.update(|f| *f = !*f)
        >
            <span class="weather-tile__icon">"🌡"</span>
            <span class="weather-tile__label">"Temperature"</span>
            <span class="weather-tile__value">
                {move || {
                    if use_fahrenheit.get() {
                        format!("{:.1}°F", celsius * 9.0 / 5.0 + 32.0)
                    } else {
                        format!("{:.1}°C", celsius)
                    }
                }}
            </span>
            // Tiny unit badge — shows the unit you'd switch TO
            <span class="temp-unit-badge">
                {move || if use_fahrenheit.get() { "°C" } else { "°F" }}
            </span>
        </button>
    }
}

fn uvi_label(uvi: f64) -> &'static str {
    match uvi as u32 {
        0..=2  => "Low",
        3..=5  => "Moderate",
        6..=7  => "High",
        8..=10 => "Very High",
        _      => "Extreme",
    }
}

fn uvi_icon(uvi: f64) -> &'static str {
    match uvi as u32 {
        0..=2  => "🌤",
        3..=5  => "☀️",
        6..=7  => "🔆",
        8..=10 => "🌞",
        _      => "⚠️",
    }
}

#[component]
fn WeatherTile<F>(icon: &'static str, label: &'static str, value: F) -> impl IntoView
where
    F: Fn() -> String + Send + Sync + 'static,
{
    view! {
        <div class="weather-tile">
            <span class="weather-tile__icon">{icon}</span>
            <span class="weather-tile__label">{label}</span>
            <span class="weather-tile__value">{value}</span>
        </div>
    }
}
