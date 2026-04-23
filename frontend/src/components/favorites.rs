use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

const STORAGE_KEY: &str = "aqi_favorites";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FavoriteLocation {
    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

// ---------------------------------------------------------------------------
// localStorage helpers
// ---------------------------------------------------------------------------

fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
}

pub fn load_favorites() -> Vec<FavoriteLocation> {
    let json = get_storage()
        .and_then(|s| s.get_item(STORAGE_KEY).ok())
        .flatten();

    match json {
        Some(ref s) => serde_json::from_str(s).unwrap_or_default(),
        None => vec![],
    }
}

pub fn persist_favorites(favs: &[FavoriteLocation]) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(favs) {
            let _ = storage.set_item(STORAGE_KEY, &json);
        }
    }
}

// ---------------------------------------------------------------------------
// FavoritesBar — a single dropdown button, not a chip list
// ---------------------------------------------------------------------------

#[component]
pub fn FavoritesBar(
    favorites: RwSignal<Vec<FavoriteLocation>>,
    /// Called when the user selects a saved location — passes (lat, lng).
    on_select: impl Fn(f64, f64) + 'static + Clone + Send,
) -> impl IntoView {
    let open = RwSignal::new(false);

    let close = move || open.set(false);

    view! {
        {move || {
            let count = favorites.get().len();
            // Clone on_select here so the outer closure stays FnMut
            let on_select = on_select.clone();
            (count > 0).then(|| view! {
                <div class="fav-dropdown">
                    // Toggle button
                    <button
                        class="fav-dropdown__toggle"
                        title="Saved locations"
                        on:click=move |_| open.update(|o| *o = !*o)
                        on:blur=move |_| {
                            // Delay so a click on a menu item fires before we close
                            spawn_local(async move {
                                TimeoutFuture::new(150).await;
                                close();
                            });
                        }
                    >
                        "★ "
                        {format!("Saved ({count})")}
                        // Chevron rotates when open
                        <svg
                            class=move || if open.get() { "fav-dropdown__chevron fav-dropdown__chevron--open" } else { "fav-dropdown__chevron" }
                            width="12" height="12" viewBox="0 0 24 24"
                            fill="none" stroke="currentColor" stroke-width="2.5"
                        >
                            <path d="m6 9 6 6 6-6"/>
                        </svg>
                    </button>

                    // Dropdown panel
                    {move || open.get().then(|| {
                        let on_select = on_select.clone();
                        let favs = favorites.get();
                        view! {
                            <ul class="fav-dropdown__menu">
                                {favs.into_iter().map(|fav| {
                                    let lat = fav.lat;
                                    let lng = fav.lng;
                                    let name = fav.name.clone();
                                    let name_rm = fav.name.clone();
                                    let on_select = on_select.clone();
                                    view! {
                                        <li class="fav-dropdown__item">
                                            <button
                                                class="fav-dropdown__item-name"
                                                on:mousedown=move |_| {
                                                    on_select(lat, lng);
                                                    open.set(false);
                                                }
                                            >
                                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7z"/>
                                                    <circle cx="12" cy="9" r="2.5"/>
                                                </svg>
                                                {name}
                                            </button>
                                            <button
                                                class="fav-dropdown__item-remove"
                                                title="Remove"
                                                on:mousedown=move |_| {
                                                    favorites.update(|v| {
                                                        v.retain(|f| f.name != name_rm);
                                                        persist_favorites(v);
                                                    });
                                                }
                                            >
                                                "×"
                                            </button>
                                        </li>
                                    }
                                }).collect::<Vec<_>>()}
                            </ul>
                        }
                    })}
                </div>
            })
        }}
    }
}
