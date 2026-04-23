use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::api::{AqiData, DailyEntry};

#[component]
pub fn AqiCard(
    data: AqiData,
    /// Whether this location is already in the user's favorites.
    #[prop(default = false)]
    is_saved: bool,
    /// Called when the user clicks the save/unsave button.
    #[prop(optional)]
    on_toggle_save: Option<Callback<()>>,
) -> impl IntoView {
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

    let icon = category.icon();
    let groups = category.sensitive_groups();
    let tips = category.recommendations();
    let sparkline_entries = data.sparkline_data();
    web_sys::console::log_1(&format!(
        "[AqiCard] Rendering section panels for {} with AQI {:?}",
        data.city.name, aqi_num
    ).into());

    // Local reactive copy of the saved state so the button updates on click
    // without requiring the whole AqiCard to re-render.
    let saved = RwSignal::new(is_saved);

    // Share: copy a formatted summary to the clipboard.
    let share_label = RwSignal::new("Share");
    let share_city  = data.city.name.clone();
    let share_aqi   = aqi_num;
    let share_cat   = label.clone();
    let on_share = move |_: web_sys::MouseEvent| {
        let text = match share_aqi {
            Some(n) => format!("AQI in {}: {} ({}) — checked via AQI Global Air Quality", share_city, n, share_cat),
            None    => format!("Air quality in {} — checked via AQI Global Air Quality", share_city),
        };
        let clipboard = web_sys::window()
            .map(|w| w.navigator().clipboard());
        if let Some(cb) = clipboard {
            let promise = cb.write_text(&text);
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                share_label.set("Copied!");
                gloo_timers::future::TimeoutFuture::new(2000).await;
                share_label.set("Share");
            });
        }
    };

    view! {
        <div class=format!("aqi-card {css_class}")>
            <div class="aqi-card__header">
                <h2 class="aqi-card__city">{data.city.name.clone()}</h2>
                <div class="aqi-card__header-right">
                    <span class="aqi-card__timestamp">"Updated: " {timestamp}</span>
                    <button class="btn-share" on:click=on_share>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/>
                            <polyline points="16 6 12 2 8 6"/>
                            <line x1="12" y1="2" x2="12" y2="15"/>
                        </svg>
                        {move || share_label.get()}
                    </button>
                    {on_toggle_save.map(|cb| view! {
                        <button
                            class=move || if saved.get() { "btn-save btn-save--saved" } else { "btn-save" }
                            title=move || if saved.get() { "Remove from favorites" } else { "Save to favorites" }
                            on:click=move |_| {
                                cb.run(());
                                saved.update(|s| *s = !*s);
                            }
                        >
                            {move || if saved.get() { "★ Saved" } else { "☆ Save" }}
                        </button>
                    })}
                </div>
            </div>

            <div class="aqi-card__body">
                <div class="aqi-card__section aqi-card__section--gauge">
                    <div class="aqi-gauge">
                        <span class="aqi-gauge__value">
                            {aqi_num.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string())}
                        </span>
                        <span class="aqi-gauge__label">AQI</span>
                    </div>
                </div>

                <div class="aqi-card__section aqi-card__section--info">
                    <div class="aqi-card__info">
                        <p class="aqi-card__category">
                            <span class="aqi-card__icon">{icon}</span>
                            {label}
                        </p>
                        {(!dominant.is_empty()).then(|| view! {
                            <p class="aqi-card__dominant">
                                "Main pollutant: " <strong>{dominant}</strong>
                            </p>
                        })}
                        <p class="aqi-card__advice">{advice}</p>
                    </div>
                </div>
            </div>

            // Health advisory panel
            <HealthAdvisory groups=groups tips=tips />

            // Multi-day sparkline (only rendered when forecast data is present)
            {(!sparkline_entries.is_empty()).then(|| view! {
                <div class="aqi-card__section aqi-card__section--sparkline">
                    <Sparkline entries=sparkline_entries today={data.time.s.clone()} />
                </div>
            })}

            <div class="aqi-card__section aqi-card__section--scale">
                <AqiScale current={aqi_num} />
            </div>
        </div>
    }
}

/// A panel showing at-risk groups and specific health recommendations.
#[component]
fn HealthAdvisory(
    groups: &'static [&'static str],
    tips: &'static [&'static str],
) -> impl IntoView {
    // If there are no groups and only a single generic tip, skip the panel.
    if groups.is_empty() && tips.len() <= 1 {
        return view! { <div></div> }.into_any();
    }

    view! {
        <div class="health-advisory">
            {(!groups.is_empty()).then(|| view! {
                <div class="health-advisory__groups-row">
                    <span class="health-advisory__label">"At-risk groups:"</span>
                    <div class="health-advisory__groups">
                        {groups.iter().map(|g| view! {
                            <span class="health-advisory__group-pill">{*g}</span>
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })}

            <ul class="health-advisory__tips">
                {tips.iter().map(|tip| view! {
                    <li class="health-advisory__tip">{*tip}</li>
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }.into_any()
}

/// A horizontal scale bar illustrating where the current AQI sits.
#[component]
fn AqiScale(current: Option<u32>) -> impl IntoView {
    let segments = [
        ("Good",      "aqi-good",           50u32,  "0–50"),
        ("Moderate",  "aqi-moderate",       100,    "51–100"),
        ("Sensitive", "aqi-sensitive",      150,    "101–150"),
        ("Unhealthy", "aqi-unhealthy",      200,    "151–200"),
        ("Very",      "aqi-very-unhealthy", 300,    "201–300"),
        ("Hazardous", "aqi-hazardous",      500,    "301+"),
    ];

    view! {
        <div class="aqi-scale">
            {segments.iter().map(|(label, cls, max, range)| {
                let label = *label;
                let cls   = *cls;
                let max   = *max;
                let range = *range;
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
                        title=format!("{label}: {range}")
                    >
                        <span class="aqi-scale__label">{label}</span>
                        <span class="aqi-scale__range">{range}</span>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// SVG sparkline showing the multi-day AQI trend from forecast data.
#[component]
fn Sparkline(entries: Vec<DailyEntry>, today: Option<String>) -> impl IntoView {
    const W: f64 = 560.0;
    const H: f64 = 72.0;
    const PAD_X: f64 = 8.0;
    const PAD_TOP: f64 = 10.0;
    const PAD_BOT: f64 = 22.0; // room for day labels

    // Keep only entries that have an avg value.
    let points: Vec<(String, f64)> = entries
        .into_iter()
        .filter_map(|e| e.avg.map(|v| (e.day, v as f64)))
        .collect();

    if points.len() < 2 {
        return view! { <div></div> }.into_any();
    }

    let min_v = points.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
    let max_v = points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
    let range = (max_v - min_v).max(1.0);
    let n = points.len() as f64;

    // Determine today's date prefix (first 10 chars of timestamp)
    let today_date = today
        .as_deref()
        .map(|s| &s[..s.len().min(10)])
        .unwrap_or("")
        .to_string();

    // Map each point to SVG coordinates
    let coords: Vec<(f64, f64, String, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, (day, val))| {
            let x = PAD_X + (i as f64 / (n - 1.0)) * (W - 2.0 * PAD_X);
            let y = PAD_TOP + (1.0 - (val - min_v) / range) * (H - PAD_TOP - PAD_BOT);
            // Short day label: "Mon", "Tue", etc. derived from the date string
            let label = day_label(day);
            (x, y, label, *val)
        })
        .collect();

    // Build polyline points string
    let polyline = coords
        .iter()
        .map(|(x, y, _, _)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");

    view! {
        <div class="sparkline">
            <div class="sparkline__title">"AQI Trend (daily avg)"</div>
            <svg
                class="sparkline__svg"
                viewBox=format!("0 0 {W} {H}")
                preserveAspectRatio="none"
                aria-hidden="true"
            >
                // Filled area under the line
                <defs>
                    <linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stop-color="currentColor" stop-opacity="0.25"/>
                        <stop offset="100%" stop-color="currentColor" stop-opacity="0.0"/>
                    </linearGradient>
                </defs>
                <polyline
                    class="sparkline__line"
                    points=polyline.clone()
                />
                // Dots at each data point
                {coords.iter().map(|(x, y, label, val)| {
                    let is_today = label == &day_label(&today_date);
                    let cx = format!("{x:.1}");
                    let cy = format!("{y:.1}");
                    let label_x = format!("{x:.1}");
                    let label_y = format!("{:.1}", H - 5.0);
                    let val_str = format!("{}", *val as i32);
                    view! {
                        <circle
                            cx=cx.clone()
                            cy=cy.clone()
                            r=if is_today { "5" } else { "3" }
                            class=if is_today { "sparkline__dot sparkline__dot--today" } else { "sparkline__dot" }
                        />
                        <text
                            x=label_x
                            y=label_y
                            class=if is_today { "sparkline__day-label sparkline__day-label--today" } else { "sparkline__day-label" }
                            text-anchor="middle"
                        >{label.clone()}</text>
                        // Value label above dot on hover — shown via CSS
                        <text
                            x=cx
                            y=format!("{:.1}", y - 8.0)
                            class="sparkline__val-label"
                            text-anchor="middle"
                        >{val_str}</text>
                    }
                }).collect::<Vec<_>>()}
            </svg>
        </div>
    }.into_any()
}

/// Derives a short weekday label ("Mon", "Wed") from a "YYYY-MM-DD" string.
fn day_label(date: &str) -> String {
    // Parse YYYY-MM-DD manually — no std date/chrono available in WASM easily
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return date.to_string();
    }
    let Ok(y) = parts[0].parse::<i32>() else { return date.to_string() };
    let Ok(m) = parts[1].parse::<i32>() else { return date.to_string() };
    let Ok(d) = parts[2].parse::<i32>() else { return date.to_string() };

    // Tomohiko Sakamoto's algorithm for day-of-week (0=Sun)
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let year = if m < 3 { y - 1 } else { y };
    let dow = (year + year / 4 - year / 100 + year / 400 + t[(m - 1) as usize] + d) % 7;
    match dow {
        0 => "Sun".to_string(),
        1 => "Mon".to_string(),
        2 => "Tue".to_string(),
        3 => "Wed".to_string(),
        4 => "Thu".to_string(),
        5 => "Fri".to_string(),
        _ => "Sat".to_string(),
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
