use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use crate::api::{fetch_aqi_search, SearchResult};

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

    // Debounced search: wait 400 ms after the user stops typing before hitting
    // the API.  We store a version counter; if it changes while we're waiting
    // we know a newer keystroke arrived and we discard the stale result.
    let version = RwSignal::new(0u32);

    let on_input = {
        move |ev: web_sys::Event| {
            let value = event_target_value(&ev);
            query.set(value.clone());
            error.set(None);

            if value.trim().is_empty() {
                results.set(vec![]);
                is_open.set(false);
                return;
            }

            // Bump version so any in-flight request with an old version is ignored.
            let v = version.get() + 1;
            version.set(v);
            is_loading.set(true);

            spawn_local(async move {
                // Debounce: 400 ms
                TimeoutFuture::new(400).await;

                // Stale check
                if version.get_untracked() != v {
                    return;
                }

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

    let on_blur = move |_: web_sys::FocusEvent| {
        // Delay close so a click on a result fires first.
        spawn_local(async move {
            TimeoutFuture::new(150).await;
            is_open.set(false);
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
                    on:blur=on_blur
                />
                {move || is_loading.get().then(|| view! {
                    <span class="search-bar__spinner" />
                })}
            </div>

            {move || error.get().map(|e| view! {
                <p class="search-bar__error">{e}</p>
            })}

            {move || {
                let res = results.get();
                (is_open.get() && !res.is_empty()).then(|| {
                    let on_select = on_select.clone();
                    view! {
                        <ul class="search-dropdown">
                            {res.into_iter().map(|r| {
                                let lat = r.station.geo.first().copied().unwrap_or(0.0);
                                let lng = r.station.geo.get(1).copied().unwrap_or(0.0);
                                let name = r.station.name.clone();
                                let country = r.station.country.clone().unwrap_or_default();
                                let aqi_display = r.aqi_number()
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| "—".to_string());
                                let on_select = on_select.clone();
                                view! {
                                    <li
                                        class="search-dropdown__item"
                                        on:mousedown=move |_| {
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
