use crate::api::{AqiData, DailyEntry};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

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
    let timestamp = data.time.s.as_deref().unwrap_or("—").to_string();

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

    // Local reactive copy of the saved state so the button updates on click
    // without requiring the whole AqiCard to re-render.
    let saved = RwSignal::new(is_saved);

    // -----------------------------------------------------------------------
    // Share dropdown state
    // -----------------------------------------------------------------------
    let share_open  = RwSignal::new(false);
    let copy_label  = RwSignal::new("Copy link");

    let aqi_str     = aqi_num.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string());
    let share_title = format!("Air Quality in {}: {} ({})", data.city.name, aqi_str, label);
    let share_body  = format!("Air quality in {}: {} ({}).", data.city.name, aqi_str, label);

    let mailto_href = format!(
        "mailto:?subject={}&body={}",
        pct(&share_title),
        pct(&share_body),
    );

    // Does the browser support navigator.share (Web Share API)?
    let has_native_share = web_sys::window()
        .map(|w| {
            let nav: wasm_bindgen::JsValue = w.navigator().into();
            js_sys::Reflect::has(&nav, &wasm_bindgen::JsValue::from_str("share"))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    view! {
        <div class=format!("aqi-card {css_class}")>
            <div class="aqi-card__header">
                <h2 class="aqi-card__city">{data.city.name.clone()}</h2>
                <div class="aqi-card__header-right">
                    <span class="aqi-card__timestamp">"Updated: " {timestamp}</span>
                    // Share dropdown
                    <div class="share-wrap">
                        <button
                            class=move || if share_open.get() { "btn-share btn-share--open" } else { "btn-share" }
                            on:click=move |_| share_open.update(|v| *v = !*v)
                            title="Share"
                        >
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/>
                                <polyline points="16 6 12 2 8 6"/>
                                <line x1="12" y1="2" x2="12" y2="15"/>
                            </svg>
                            "Share"
                        </button>
                        // Reactive wrapper — only captures Copy/Clone values so the
                        // closure stays Fn (not FnOnce).
                        {move || share_open.get().then(|| view! {
                            <ShareMenu
                                share_open=share_open
                                copy_label=copy_label
                                mailto_href=mailto_href.clone()
                                share_title=share_title.clone()
                                share_body=share_body.clone()
                                has_native_share=has_native_share
                            />
                        })}
                    </div>
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
fn HealthAdvisory(groups: &'static [&'static str], tips: &'static [&'static str]) -> impl IntoView {
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
    }
    .into_any()
}

/// A horizontal scale bar illustrating where the current AQI sits.
#[component]
fn AqiScale(current: Option<u32>) -> impl IntoView {
    let segments = [
        ("Good", "aqi-good", 50u32, "0–50"),
        ("Moderate", "aqi-moderate", 100, "51–100"),
        ("Sensitive", "aqi-sensitive", 150, "101–150"),
        ("Unhealthy", "aqi-unhealthy", 200, "151–200"),
        ("Very", "aqi-very-unhealthy", 300, "201–300"),
        ("Hazardous", "aqi-hazardous", 500, "301+"),
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
    let max_v = points
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max_v - min_v).max(1.0);
    let n = points.len() as f64;

    // Determine today's date prefix (first 10 chars of timestamp)
    let today_date = today
        .as_deref()
        .map(|s| &s[..s.len().min(10)])
        .unwrap_or("")
        .to_string();

    // Map each point to SVG coordinates
    let coords: Vec<(f64, f64, String, f64, String)> = points
        .iter()
        .enumerate()
        .map(|(i, (day, val))| {
            let x = PAD_X + (i as f64 / (n - 1.0)) * (W - 2.0 * PAD_X);
            let y = PAD_TOP + (1.0 - (val - min_v) / range) * (H - PAD_TOP - PAD_BOT);
            // Short day label: "Mon", "Tue", etc. derived from the date string
            let label = day_label(day);
            (x, y, label, *val, day.clone())
        })
        .collect();

    // Build polyline points string
    let polyline = coords
        .iter()
        .map(|(x, y, _, _, _)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");

    view! {
        <div class="sparkline">
            <div class="sparkline__title">"AQI Trend (daily avg)"</div>
            <svg
                class="sparkline__svg"
                viewBox=format!("0 0 {W} {H}")
                preserveAspectRatio="xMidYMid meet"
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
                {coords.iter().map(|(x, y, label, val, day)| {
                    let is_today = day == &today_date;
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
    let Ok(y) = parts[0].parse::<i32>() else {
        return date.to_string();
    };
    let Ok(m) = parts[1].parse::<i32>() else {
        return date.to_string();
    };
    let Ok(d) = parts[2].parse::<i32>() else {
        return date.to_string();
    };

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

/// Dropdown share menu rendered when the Share button is active.
/// Defined as a separate component so its event handlers are set up once
/// (not inside a reactive closure) which avoids FnOnce capture issues.
#[component]
fn ShareMenu(
    share_open: RwSignal<bool>,
    copy_label: RwSignal<&'static str>,
    mailto_href: String,
    share_title: String,
    share_body: String,
    has_native_share: bool,
) -> impl IntoView {
    // --- Copy link ---
    let on_copy_link = move |_: web_sys::MouseEvent| {
        share_open.set(false);
        let url = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .unwrap_or_default();
        if let Some(cb) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let promise = cb.write_text(&url);
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                copy_label.set("Copied!");
                gloo_timers::future::TimeoutFuture::new(2000).await;
                copy_label.set("Copy link");
            });
        }
    };

    // --- Native share (Web Share API) ---
    let native_title = share_title.clone();
    let native_body  = share_body.clone();
    let on_native_share = move |_: web_sys::MouseEvent| {
        share_open.set(false);
        let title = native_title.clone();
        let text  = native_body.clone();
        let url   = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .unwrap_or_default();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let nav_val: wasm_bindgen::JsValue = window.navigator().into();
                let data = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&data, &"title".into(), &wasm_bindgen::JsValue::from_str(&title));
                let _ = js_sys::Reflect::set(&data, &"text".into(),  &wasm_bindgen::JsValue::from_str(&text));
                let _ = js_sys::Reflect::set(&data, &"url".into(),   &wasm_bindgen::JsValue::from_str(&url));
                if let Ok(share_fn) = js_sys::Reflect::get(&nav_val, &"share".into()) {
                    if let Ok(f) = share_fn.dyn_into::<js_sys::Function>() {
                        if let Ok(pv) = f.call1(&nav_val, &data) {
                            if let Ok(p) = pv.dyn_into::<js_sys::Promise>() {
                                let _ = wasm_bindgen_futures::JsFuture::from(p).await;
                            }
                        }
                    }
                }
            }
        });
    };

    view! {
        <div class="share-dropdown">
            // Copy link
            <button class="share-dropdown__item" on:click=on_copy_link>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                </svg>
                {move || copy_label.get()}
            </button>

            // Email via mailto:
            <a
                class="share-dropdown__item"
                href=mailto_href
                on:click=move |_| share_open.set(false)
            >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
                    <polyline points="22,6 12,13 2,6"/>
                </svg>
                "Email"
            </a>

            // Native share sheet — only rendered when navigator.share is available
            {has_native_share.then(|| view! {
                <button class="share-dropdown__item" on:click=on_native_share>
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="18" cy="5" r="3"/>
                        <circle cx="6" cy="12" r="3"/>
                        <circle cx="18" cy="19" r="3"/>
                        <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/>
                        <line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/>
                    </svg>
                    "Share..."
                </button>
            })}
        </div>
    }
}

/// Percent-encode a string for use inside a `mailto:` query parameter.
/// Only encodes characters that are meaningful inside URLs / query strings;
/// non-ASCII letters (e.g. accented city names) are left as-is.
pub(crate) fn pct(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            ' '  => "%20".chars().collect::<Vec<_>>(),
            '\n' => "%0A".chars().collect(),
            '\r' => "%0D".chars().collect(),
            '&'  => "%26".chars().collect(),
            '?'  => "%3F".chars().collect(),
            '#'  => "%23".chars().collect(),
            '+'  => "%2B".chars().collect(),
            '"'  => "%22".chars().collect(),
            _    => vec![c],
        })
        .collect()
}

fn prettify_pollutant(raw: &str) -> &str {
    match raw {
        "pm25" => "PM₂.₅",
        "pm10" => "PM₁₀",
        "no2" => "NO₂",
        "o3" => "O₃",
        "co" => "CO",
        "so2" => "SO₂",
        other => other,
    }
}
