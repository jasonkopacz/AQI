mod api;
mod components;

#[cfg(test)]
mod test;

use api::{fetch_aqi_by_geo, AqiData};
use components::{
    load_favorites, persist_favorites, AqiCard, FavoriteLocation, FavoritesBar, ForecastPanel,
    PollutantsGrid, SearchBar,
};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

// ---------------------------------------------------------------------------
// Geolocation helper
// Wraps the callback-based browser Geolocation API in an async Future using a
// JS Promise so we can await it cleanly from Rust.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// URL helpers — encode the loaded city into the address bar so users can
// copy or bookmark the link, and read it back on page load.
// ---------------------------------------------------------------------------

/// Reads `?lat=X&lng=Y` from the current URL.  Returns `None` if absent or
/// if either value cannot be parsed as a float.
fn parse_url_coords() -> Option<(f64, f64)> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let search = search.trim_start_matches('?');
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(search).ok()?;
    let lat = params.get("lat")?.parse::<f64>().ok()?;
    let lng = params.get("lng")?.parse::<f64>().ok()?;
    Some((lat, lng))
}

/// Pushes `?lat=…&lng=…&city=…` into the browser history without reloading.
pub fn push_url_state(lat: f64, lng: f64, city: &str) {
    // Basic percent-encoding for characters that break URL params.
    let encoded: String = city
        .chars()
        .flat_map(|c| match c {
            ' ' => vec!['%', '2', '0'],
            '&' => vec!['%', '2', '6'],
            '#' => vec!['%', '2', '3'],
            '?' => vec!['%', '3', 'F'],
            '+' => vec!['%', '2', 'B'],
            c => vec![c],
        })
        .collect();
    let url = format!("?lat={:.4}&lng={:.4}&city={}", lat, lng, encoded);
    if let Some(window) = web_sys::window() {
        if let Ok(history) = window.history() {
            let _ = history.push_state_with_url(
                &wasm_bindgen::JsValue::NULL,
                "",
                Some(&url),
            );
        }
    }
}

async fn get_browser_location() -> Result<(f64, f64), String> {
    use js_sys::Promise;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or("No browser window")?;
    let geolocation = window
        .navigator()
        .geolocation()
        .map_err(|_| "Geolocation not supported".to_string())?;

    // Build a Promise that resolves with [lat, lng] or rejects with a message.
    let promise = Promise::new(&mut |resolve, reject| {
        let on_success = Closure::once(move |pos: web_sys::Position| {
            let coords = pos.coords();
            let arr = js_sys::Array::new();
            arr.push(&JsValue::from_f64(coords.latitude()));
            arr.push(&JsValue::from_f64(coords.longitude()));
            let _ = resolve.call1(&JsValue::UNDEFINED, &arr);
        });

        let on_error = Closure::once(move |err: web_sys::PositionError| {
            let msg = JsValue::from_str(&format!(
                "Geolocation error {}: {}",
                err.code(),
                err.message()
            ));
            let _ = reject.call1(&JsValue::UNDEFINED, &msg);
        });

        // Give the browser 10 seconds to resolve; after that the error
        // callback fires automatically and we fall through to Idle state.
        let opts = web_sys::PositionOptions::new();
        opts.set_timeout(10_000);

        let _ = geolocation.get_current_position_with_error_callback_and_options(
            on_success.as_ref().unchecked_ref(),
            Some(on_error.as_ref().unchecked_ref()),
            &opts,
        );

        // Leak the closures so they live long enough for the JS callback to fire.
        on_success.forget();
        on_error.forget();
    });

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| {
            e.as_string()
                .unwrap_or_else(|| "Unknown geolocation error".to_string())
        })?;

    let arr: js_sys::Array = result
        .dyn_into()
        .map_err(|_| "Unexpected geolocation result format".to_string())?;

    let lat = arr.get(0).as_f64().ok_or("Missing latitude")?;
    let lng = arr.get(1).as_f64().ok_or("Missing longitude")?;

    Ok((lat, lng))
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum AppView {
    /// The very first render — trying to detect location automatically.
    Detecting,
    /// Location detected / coordinates chosen, loading AQI.
    Loading,
    /// AQI loaded successfully.
    Loaded(Box<AqiData>),
    /// Something went wrong.
    Error(String),
    /// User denied geolocation and hasn't searched yet.
    Idle,
}

// ---------------------------------------------------------------------------
// Root component
// ---------------------------------------------------------------------------

#[component]
fn App() -> impl IntoView {
    let view_state = RwSignal::new(AppView::Detecting);
    let favorites = RwSignal::new(load_favorites());
    let is_light = RwSignal::new(false);
    let request_version = RwSignal::new(0u64);

    // Shared helper: given coordinates, fetch AQI and update the view.
    let load_aqi = move |lat: f64, lng: f64| {
        let current_request = request_version.get_untracked() + 1;
        request_version.set(current_request);
        view_state.set(AppView::Loading);
        spawn_local(async move {
            match fetch_aqi_by_geo(lat, lng).await {
                Ok(data) if request_version.get_untracked() == current_request => {
                    push_url_state(lat, lng, &data.city.name);
                    view_state.set(AppView::Loaded(Box::new(data)));
                }
                Err(e) if request_version.get_untracked() == current_request => {
                    view_state.set(AppView::Error(e));
                }
                Ok(_) => {}
                Err(_) => {}
            }
        });
    };

    // On mount: check URL params first, then fall back to Geolocation API.
    // Effect::new runs once after the first render.
    {
        Effect::new(move |_| {
            // If the page URL already has ?lat=…&lng=… (e.g. shared link), load
            // that location directly without asking for browser geolocation.
            if let Some((lat, lng)) = parse_url_coords() {
                load_aqi(lat, lng);
                return;
            }
            spawn_local(async move {
                match get_browser_location().await {
                    Ok((lat, lng)) => load_aqi(lat, lng),
                    Err(_) => {
                        // User denied or browser unsupported — show idle state.
                        view_state.set(AppView::Idle);
                    }
                }
            });
        });
    }

    view! {
        <div class=move || {
            let aqi_suffix = match view_state.get() {
                AppView::Loaded(data) => format!(" app--{}", data.category().css_class()),
                _ => String::new(),
            };
            if is_light.get() {
                format!("app theme-light{aqi_suffix}")
            } else {
                format!("app{aqi_suffix}")
            }
        }>
            <header class="header">
                <div class="header__brand">
                    <img
                        class="header__brand-logo"
                        src="/icons/logo-brand.png"
                        alt="AQI — Air Quality Info"
                        width="620"
                        height="488"
                    />
                </div>

                <div class="header__controls">
                    <SearchBar on_select=move |lat, lng| load_aqi(lat, lng) />
                    <FavoritesBar favorites=favorites on_select=move |lat, lng| load_aqi(lat, lng) />

                    <button
                        class="btn-theme"
                        title="Toggle light/dark theme"
                        on:click=move |_| is_light.update(|v| *v = !*v)
                    >
                        {move || if is_light.get() { "🌙" } else { "☀️" }}
                    </button>

                    <button
                        class="btn-locate"
                        title="Use my current location"
                        on:click=move |_| {
                            spawn_local(async move {
                                view_state.set(AppView::Detecting);
                                match get_browser_location().await {
                                    Ok((lat, lng)) => load_aqi(lat, lng),
                                    Err(e) => view_state.set(AppView::Error(e)),
                                }
                            });
                        }
                    >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="3"/>
                            <path d="M12 2v3M12 19v3M2 12h3M19 12h3"/>
                            <path d="M12 2a10 10 0 1 0 0 20A10 10 0 0 0 12 2z" stroke-dasharray="3 3"/>
                        </svg>
                        "My Location"
                    </button>
                </div>
            </header>

            <main class="main">
                {move || match view_state.get() {
                    AppView::Detecting => view! {
                        <div class="status-panel">
                            <div class="spinner" />
                            <p class="status-panel__msg">"Detecting your location…"</p>
                            <p class="status-panel__hint">
                                "If prompted, please allow location access."
                            </p>
                            <button
                                class="btn-skip"
                                on:click=move |_| view_state.set(AppView::Idle)
                            >
                                "Skip — search manually"
                            </button>
                        </div>
                    }.into_any(),

                    AppView::Loading => view! {
                        <div class="dashboard">
                            // AQI card skeleton
                            <div class="skeleton-card">
                                <div class="skeleton-card__header">
                                    <div class="skel skel--title" />
                                    <div class="skel skel--chip" />
                                </div>
                                <div class="skeleton-card__body">
                                    <div class="skel skel--circle" />
                                    <div class="skeleton-card__lines">
                                        <div class="skel skel--line skel--line-lg" />
                                        <div class="skel skel--line skel--line-md" />
                                        <div class="skel skel--line skel--line-sm" />
                                    </div>
                                </div>
                                <div class="skel skel--bar" />
                            </div>
                            // Pollutants skeleton
                            <div class="skeleton-card">
                                <div class="skel skel--line skel--line-sm" style="margin-bottom:1rem" />
                                <div class="skeleton-card__grid">
                                    <div class="skel skel--tile" />
                                    <div class="skel skel--tile" />
                                    <div class="skel skel--tile" />
                                    <div class="skel skel--tile" />
                                    <div class="skel skel--tile" />
                                    <div class="skel skel--tile" />
                                </div>
                            </div>
                        </div>
                    }.into_any(),

                    AppView::Loaded(data) => {
                        let data = *data;
                        let iaqi = data.iaqi.clone();
                        let city_name = data.city.name.clone();
                        let lat = data.city.geo.first().copied().unwrap_or(0.0);
                        let lng = data.city.geo.get(1).copied().unwrap_or(0.0);
                        let is_saved = favorites.get_untracked()
                            .iter()
                            .any(|f| f.lat == lat && f.lng == lng);
                        let city_for_save = city_name.clone();
                        let on_toggle_save = Callback::new(move |_: ()| {
                            favorites.update(|v| {
                                if let Some(pos) =
                                    v.iter().position(|f| f.lat == lat && f.lng == lng)
                                {
                                    v.remove(pos);
                                } else {
                                    v.push(FavoriteLocation {
                                        name: city_for_save.clone(),
                                        lat,
                                        lng,
                                    });
                                }
                                persist_favorites(v);
                            });
                        });
                        let forecast_entries = data.forecast_day_details();
                        let forecast_today = data.time.s.clone();
                        let uvi = data.uvi_today();
                        view! {
                            <div class="dashboard">
                                <AqiCard
                                    data=data
                                    is_saved=is_saved
                                    on_toggle_save=on_toggle_save
                                />
                                <ForecastPanel
                                    entries=forecast_entries
                                    today=forecast_today
                                />
                                <PollutantsGrid iaqi=iaqi uvi=uvi />
                            </div>
                        }.into_any()
                    },

                    AppView::Error(msg) => {
                        view! {
                            <div class="status-panel status-panel--error">
                                <p class="status-panel__icon">"⚠️"</p>
                                <p class="status-panel__msg">{msg}</p>
                                <p class="status-panel__hint">
                                    "Try searching for a city above, or retry your location."
                                </p>
                                <div class="status-panel__actions">
                                    <button
                                        class="btn-retry"
                                        on:click=move |_| {
                                            spawn_local(async move {
                                                view_state.set(AppView::Detecting);
                                                match get_browser_location().await {
                                                    Ok((lat, lng)) => load_aqi(lat, lng),
                                                    Err(_) => view_state.set(AppView::Idle),
                                                }
                                            });
                                        }
                                    >
                                        "↺ Retry location"
                                    </button>
                                    <button
                                        class="btn-skip"
                                        on:click=move |_| view_state.set(AppView::Idle)
                                    >
                                        "Search manually"
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    },

                    AppView::Idle => view! {
                        <div class="status-panel">
                            <p class="status-panel__icon">"🔍"</p>
                            <p>"Search for a city to see its air quality."</p>
                            <p class="status-panel__hint">
                                "Or use the My Location button to enable automatic detection."
                            </p>
                        </div>
                    }.into_any(),
                }}
            </main>

            <footer class="footer">
                <span class="footer__name">"Jason Kopacz"</span>
                <span class="footer__sep">"·"</span>
                <a href="mailto:jtkopacz@gmail.com">"jtkopacz@gmail.com"</a>
                <span class="footer__sep">"·"</span>
                <a href="https://github.com/jasonkopacz" target="_blank" rel="noreferrer">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.009-.868-.013-1.703-2.782.604-3.369-1.341-3.369-1.341-.454-1.155-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0 1 12 6.836a9.59 9.59 0 0 1 2.504.337c1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.202 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.744 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z"/>
                    </svg>
                    "GitHub"
                </a>
                <span class="footer__sep">"·"</span>
                <a href="https://www.linkedin.com/in/jasonkopacz/" target="_blank" rel="noreferrer">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433a2.062 2.062 0 0 1-2.063-2.065 2.064 2.064 0 1 1 2.063 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"/>
                    </svg>
                    "LinkedIn"
                </a>
            </footer>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Entry point — Trunk calls `main()` via wasm-bindgen
// ---------------------------------------------------------------------------

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    // Forward Rust panics to the browser console.
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
