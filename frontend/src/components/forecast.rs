use leptos::prelude::*;
use crate::api::{AqiCategory, DailyEntry, ForecastDayDetails};

/// Horizontal row of day cards showing the multi-day AQI forecast.
/// Entries should already be the best available series (pm25 → pm10 → o3).
#[component]
pub fn ForecastPanel(
    entries: Vec<ForecastDayDetails>,
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
            let avg = e.primary.avg?;
            Some(ForecastCard {
                label: day_label(&e.day),
                date: e.day.clone(),
                avg,
                min: e.primary.min,
                max: e.primary.max,
                pm25: e.pm25,
                pm10: e.pm10,
                o3: e.o3,
                uvi: e.uvi,
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
                    let display_day = if card.is_today {
                        "Today".to_string()
                    } else {
                        card.label.clone()
                    };
                    let min_text = card.min.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
                    let max_text = card.max.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
                    let pm25_text = format_measurement_row(&card.pm25);
                    let pm10_text = format_measurement_row(&card.pm10);
                    let o3_text = format_measurement_row(&card.o3);
                    let uvi_text = format_measurement_row(&card.uvi);
                    let status = if card.is_today {
                        "Today"
                    } else if card.is_past {
                        "Past"
                    } else {
                        "Upcoming"
                    };

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
                        <div class="forecast-card-wrap">
                            <div class=outer_class>
                                <span class="forecast-card__day">{display_day.clone()}</span>
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
                            <div class="forecast-card-tooltip" role="tooltip">
                                <div class="forecast-card-tooltip__row"><strong>"Day:"</strong> <span>{display_day}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"Date:"</strong> <span>{card.date}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"AQI (Primary):"</strong> <span>{card.avg}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"Category:"</strong> <span>{cat_label.clone()}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"Min:"</strong> <span>{min_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"Max:"</strong> <span>{max_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"PM2.5:"</strong> <span>{pm25_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"PM10:"</strong> <span>{pm10_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"O3:"</strong> <span>{o3_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"UVI:"</strong> <span>{uvi_text}</span></div>
                                <div class="forecast-card-tooltip__row"><strong>"Status:"</strong> <span>{status}</span></div>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }.into_any()
}

struct ForecastCard {
    label: String,
    date: String,
    avg: i32,
    min: Option<i32>,
    max: Option<i32>,
    pm25: Option<DailyEntry>,
    pm10: Option<DailyEntry>,
    o3: Option<DailyEntry>,
    uvi: Option<DailyEntry>,
    is_today: bool,
    is_past: bool,
}

fn format_measurement_row(entry: &Option<DailyEntry>) -> String {
    match entry {
        Some(value) => {
            let avg = value
                .avg
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string());
            let min = value
                .min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string());
            let max = value
                .max
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string());
            format!("avg {avg}, min {min}, max {max}")
        }
        None => "No data".to_string(),
    }
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
