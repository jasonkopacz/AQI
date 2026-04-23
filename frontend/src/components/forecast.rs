use leptos::prelude::*;
use crate::api::{AqiCategory, DailyEntry};

/// Horizontal row of day cards showing the multi-day AQI forecast.
/// Entries should already be the best available series (pm25 → pm10 → o3).
#[component]
pub fn ForecastPanel(
    entries: Vec<DailyEntry>,
    /// The station's local date string "YYYY-MM-DD HH:MM:SS" (from TimeInfo.s).
    today: Option<String>,
) -> impl IntoView {
    if entries.is_empty() {
        return view! { <div></div> }.into_any();
    }

    let today_date = today
        .as_deref()
        .map(|s| &s[..s.len().min(10)])
        .unwrap_or("")
        .to_string();

    // Keep entries that have at least an avg value; include past 1 day + all future
    let cards: Vec<ForecastCard> = entries
        .into_iter()
        .filter_map(|e| {
            let avg = e.avg?;
            Some(ForecastCard {
                label: day_label(&e.day),
                avg,
                min: e.min,
                max: e.max,
                is_today: e.day == today_date,
                is_past: e.day < today_date,
            })
        })
        .collect();

    if cards.len() < 2 {
        return view! { <div></div> }.into_any();
    }

    // For the min/max range bar: find global min/max across all cards
    let global_min = cards.iter().filter_map(|c| c.min).min().unwrap_or(0) as f64;
    let global_max = cards.iter().filter_map(|c| c.max).max().unwrap_or(500) as f64;
    let range = (global_max - global_min).max(1.0);

    view! {
        <div class="forecast">
            <h3 class="forecast__title">"Daily Air Quality Forecast"</h3>
            <div class="forecast__row">
                {cards.into_iter().map(|card| {
                    let category = AqiCategory::from(card.avg as u32);
                    let color_class = category.css_class().to_string();
                    let cat_label = category.label().to_string();

                    // Min/max bar: position as % within global range
                    let bar_left = card.min
                        .map(|m| (m as f64 - global_min) / range * 100.0)
                        .unwrap_or(0.0);
                    let bar_width = match (card.min, card.max) {
                        (Some(mn), Some(mx)) => (mx as f64 - mn as f64) / range * 100.0,
                        _ => 0.0,
                    };

                    let outer_class = if card.is_today {
                        format!("forecast-card forecast-card--today {color_class}")
                    } else if card.is_past {
                        format!("forecast-card forecast-card--past {color_class}")
                    } else {
                        format!("forecast-card {color_class}")
                    };

                    view! {
                        <div class=outer_class title=cat_label.clone()>
                            <span class="forecast-card__day">
                                {if card.is_today { "Today".to_string() } else { card.label }}
                            </span>
                            <span class="forecast-card__aqi">{card.avg}</span>
                            <span class="forecast-card__cat">{cat_label.clone()}</span>
                            // Min/max range bar
                            {(bar_width > 0.0).then(|| view! {
                                <div class="forecast-card__range">
                                    <div
                                        class="forecast-card__range-bar"
                                        style=format!(
                                            "left:{bar_left:.1}%;width:{bar_width:.1}%"
                                        )
                                    />
                                </div>
                            })}
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }.into_any()
}

struct ForecastCard {
    label: String,
    avg: i32,
    min: Option<i32>,
    max: Option<i32>,
    is_today: bool,
    is_past: bool,
}

fn day_label(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 { return date.to_string(); }
    let Ok(y) = parts[0].parse::<i32>() else { return date.to_string() };
    let Ok(m) = parts[1].parse::<i32>() else { return date.to_string() };
    let Ok(d) = parts[2].parse::<i32>() else { return date.to_string() };
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let year = if m < 3 { y - 1 } else { y };
    let dow = (year + year/4 - year/100 + year/400 + t[(m-1) as usize] + d) % 7;
    match dow {
        0 => "Sun", 1 => "Mon", 2 => "Tue", 3 => "Wed",
        4 => "Thu", 5 => "Fri", _ => "Sat",
    }.to_string()
}
