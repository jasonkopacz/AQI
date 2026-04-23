use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use crate::api::{fetch_aqi_search, SearchResult};

const RECENT_KEY: &str = "aqi_recent";
const MAX_RECENT: usize = 5;

// ---------------------------------------------------------------------------
// Recent-search helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentLocation {
    pub name: String,
    pub lat:  f64,
    pub lng:  f64,
}

fn load_recent() -> Vec<RecentLocation> {
    let json = web_sys::window()
        .and_then(|w| w.local_storage().ok()).flatten()
        .and_then(|s| s.get_item(RECENT_KEY).ok()).flatten();
    match json {
        Some(ref s) => serde_json::from_str(s).unwrap_or_default(),
        None => vec![],
    }
}

fn push_recent(loc: RecentLocation) -> Vec<RecentLocation> {
    let mut list = load_recent();
    list.retain(|r| r.name != loc.name);
    list.insert(0, loc);
    list.truncate(MAX_RECENT);
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
        if let Ok(json) = serde_json::to_string(&list) {
            let _ = s.set_item(RECENT_KEY, &json);
        }
    }
    list
}

// ---------------------------------------------------------------------------
// SearchBar component
// ---------------------------------------------------------------------------

#[component]
pub fn SearchBar(
    /// Called when the user selects a result — passes (lat, lng).
    on_select: impl Fn(f64, f64) + 'static + Clone + Send,
) -> impl IntoView {
    let query      = RwSignal::new(String::new());
    let results    = RwSignal::new(Vec::<SearchResult>::new());
    let is_open    = RwSignal::new(false);
    let is_loading = RwSignal::new(false);
    let error      = RwSignal::new(None::<String>);
    let recent     = RwSignal::new(load_recent());
    let show_recent = RwSignal::new(false);

    let version = RwSignal::new(0u32);

    let on_input = {
        move |ev: web_sys::Event| {
            let value = event_target_value(&ev);
            query.set(value.clone());
            error.set(None);

            if value.trim().is_empty() {
                results.set(vec![]);
                is_open.set(false);
                // Show recent searches when input is cleared
                show_recent.set(!recent.get_untracked().is_empty());
                return;
            }

            show_recent.set(false);
            let v = version.get() + 1;
            version.set(v);
            is_loading.set(true);

            spawn_local(async move {
                TimeoutFuture::new(400).await;
                if version.get_untracked() != v { return; }

                match fetch_aqi_search(&value).await {
                    Ok(data) => {
                        if version.get_untracked() == v {
                            results.set(data);
                            is_open.set(true);
                            is_loading.set(false);
                        }
                    }
                    Err(e) => {
                        if version.get_untracked() == v {
                            error.set(Some(e));
                            is_loading.set(false);
                        }
                    }
                }
            });
        }
    };

    let on_focus = move |_: web_sys::FocusEvent| {
        if query.get_untracked().trim().is_empty() {
            show_recent.set(!recent.get_untracked().is_empty());
        }
    };

    let on_blur = move |_: web_sys::FocusEvent| {
        spawn_local(async move {
            TimeoutFuture::new(150).await;
            is_open.set(false);
            show_recent.set(false);
        });
    };

    view! {
        <div class="search-bar">
            <div class="search-bar__input-wrap">
                <svg class="search-bar__icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="m21 21-4.35-4.35"/>
                </svg>
                <input
                    type="text"
                    class="search-bar__input"
                    placeholder="Search city or monitoring station…"
                    prop:value=move || query.get()
                    on:input=on_input
                    on:focus=on_focus
                    on:blur=on_blur
                />
                {move || is_loading.get().then(|| view! {
                    <span class="search-bar__spinner" />
                })}
            </div>

            {move || error.get().map(|e| view! {
                <p class="search-bar__error">{e}</p>
            })}

            // Recent searches panel (shown when input is focused + empty)
            {
            let on_select_recent = on_select.clone();
            move || {
                let recents = recent.get();
                (show_recent.get() && !recents.is_empty()).then(|| {
                    let on_select = on_select_recent.clone();
                    view! {
                        <ul class="search-dropdown">
                            <li class="search-dropdown__section-label">"Recent"</li>
                            {recents.into_iter().map(|r| {
                                let lat  = r.lat;
                                let lng  = r.lng;
                                let name = r.name.clone();
                                let on_select = on_select.clone();
                                view! {
                                    <li
                                        class="search-dropdown__item"
                                        on:mousedown=move |_| {
                                            on_select(lat, lng);
                                            show_recent.set(false);
                                            query.set(String::new());
                                        }
                                    >
                                        <span class="search-dropdown__name">
                                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="flex-shrink:0;opacity:0.5">
                                                <circle cx="12" cy="12" r="10"/>
                                                <polyline points="12 6 12 12 16 14"/>
                                            </svg>
                                            {name}
                                        </span>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    }
                })
            }}

            // Live search results dropdown

            {move || {
                let res = results.get();
                (is_open.get() && !res.is_empty()).then(|| {
                    let on_select = on_select.clone();
                    view! {
                        <ul class="search-dropdown">
                            {res.into_iter().map(|r| {
                                let lat     = r.station.geo.first().copied().unwrap_or(0.0);
                                let lng     = r.station.geo.get(1).copied().unwrap_or(0.0);
                                let name    = r.station.name.clone();
                                let country = r.station.country.clone().unwrap_or_default();
                                let aqi_display = r.aqi_number()
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| "—".to_string());
                                let on_select = on_select.clone();
                                let save_name = name.clone();
                                view! {
                                    <li
                                        class="search-dropdown__item"
                                        on:mousedown=move |_| {
                                            // Save to recents before calling on_select
                                            let updated = push_recent(RecentLocation {
                                                name: save_name.clone(),
                                                lat,
                                                lng,
                                            });
                                            recent.set(updated);
                                            on_select(lat, lng);
                                            is_open.set(false);
                                            query.set(String::new());
                                            results.set(vec![]);
                                        }
                                    >
                                        <span class="search-dropdown__name">{name}</span>
                                        <span class="search-dropdown__meta">
                                            {(!country.is_empty()).then(|| view! {
                                                <span class="search-dropdown__country">{country}</span>
                                            })}
                                            <span class="search-dropdown__aqi">"AQI: " {aqi_display}</span>
                                        </span>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    }
                })
            }}
        </div>
    }
}
