//! Weather & Moon Phase: live Open-Meteo weather forecast, 24-hour
//! temperature sparkline, wind/humidity stats, and astronomical lunar phase
//! calculator (inspired by NotchPixel's WeatherService & Moon widget).

use crate::kit;
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::{icons, widgets};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
const SYNODIC_DAYS: f64 = 29.530588853;
/// Reference new moon: 2000-01-06 18:14 UTC (Unix timestamp 947182440).
const REF_NEW_MOON_UNIX: f64 = 947_182_440.0;

#[derive(Clone, Copy, Debug)]
struct CityPreset {
    name: &'static str,
    lat: f64,
    lon: f64,
}

const CITIES: &[CityPreset] = &[
    CityPreset { name: "Auto", lat: 0.0, lon: 0.0 },
    CityPreset { name: "Riyadh", lat: 24.7136, lon: 46.6753 },
    CityPreset { name: "Makkah", lat: 21.3891, lon: 39.8579 },
    CityPreset { name: "Cairo", lat: 30.0444, lon: 31.2357 },
    CityPreset { name: "Dubai", lat: 25.2048, lon: 55.2708 },
    CityPreset { name: "Khartoum", lat: 15.5007, lon: 32.5599 },
    CityPreset { name: "Doha", lat: 25.2854, lon: 51.5310 },
    CityPreset { name: "London", lat: 51.5072, lon: -0.1276 },
];

#[derive(Clone, Debug)]
struct WeatherData {
    city: String,
    temp_c: f32,
    feels_c: f32,
    high_c: f32,
    low_c: f32,
    humidity: u32,
    wind_kmh: f32,
    code: u32,
    hourly: Vec<f32>,
    #[allow(dead_code)]
    updated_at: Instant,
}

impl Default for WeatherData {
    fn default() -> Self {
        Self {
            city: "Riyadh".into(),
            temp_c: 26.0,
            feels_c: 25.0,
            high_c: 31.0,
            low_c: 20.0,
            humidity: 35,
            wind_kmh: 11.0,
            code: 0,
            hourly: vec![
                21.0, 20.5, 20.0, 19.8, 20.2, 21.5, 23.0, 25.0, 27.0, 28.5, 29.8, 30.5,
                31.0, 30.8, 30.0, 29.0, 27.5, 26.0, 24.8, 23.8, 23.0, 22.5, 22.0, 21.5,
            ],
            updated_at: Instant::now(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonInfo {
    pub age_days: f32,
    pub illumination: f32,
    pub phase_name: &'static str,
}

pub fn current_moon_info() -> MoonInfo {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(REF_NEW_MOON_UNIX);
    moon_info_at(unix)
}

pub fn moon_info_at(unix_secs: f64) -> MoonInfo {
    let days_since = (unix_secs - REF_NEW_MOON_UNIX) / 86_400.0;
    let cycle = days_since.rem_euclid(SYNODIC_DAYS);
    let frac = cycle / SYNODIC_DAYS;
    let illum = 0.5 * (1.0 - (std::f64::consts::TAU * frac).cos());
    let phase_name = match (frac * 8.0).round() as u32 % 8 {
        0 => "New Moon",
        1 => "Waxing Crescent",
        2 => "First Quarter",
        3 => "Waxing Gibbous",
        4 => "Full Moon",
        5 => "Waning Gibbous",
        6 => "Last Quarter",
        _ => "Waning Crescent",
    };
    MoonInfo {
        age_days: cycle as f32,
        illumination: (illum * 100.0) as f32,
        phase_name,
    }
}

fn condition_label_and_icon(code: u32) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear sky", icons::SUN),
        1 | 2 => ("Partly cloudy", icons::CLOUD_SUN),
        3 => ("Overcast", icons::CLOUD),
        45 | 48 => ("Foggy", icons::CLOUD_FOG),
        51..=67 | 80..=82 => ("Rain", icons::CLOUD_RAIN),
        71..=77 | 85 | 86 => ("Snow", icons::CLOUD_SNOW),
        95..=99 => ("Thunderstorm", icons::CLOUD_LIGHTNING),
        _ => ("Fair", icons::CLOUD_SUN),
    }
}

pub struct Weather {
    data: Arc<Mutex<WeatherData>>,
    fetching: Arc<Mutex<bool>>,
    selected_city: usize,
    fahrenheit: bool,
    last_fetch: Option<Instant>,
    open_page: bool,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(WeatherData::default())),
            fetching: Arc::new(Mutex::new(false)),
            selected_city: 0,
            fahrenheit: false,
            last_fetch: None,
            open_page: false,
        }
    }
}

impl Weather {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.trigger_fetch();
        s
    }

    fn format_temp(&self, c: f32) -> String {
        if self.fahrenheit {
            format!("{:.0}°F", c * 1.8 + 32.0)
        } else {
            format!("{:.0}°C", c)
        }
    }

    fn trigger_fetch(&mut self) {
        if let Ok(mut f) = self.fetching.lock() {
            if *f {
                return;
            }
            *f = true;
        }
        self.last_fetch = Some(Instant::now());
        let preset = CITIES[self.selected_city.min(CITIES.len() - 1)];
        let data_arc = Arc::clone(&self.data);
        let fetching_arc = Arc::clone(&self.fetching);

        std::thread::spawn(move || {
            let (city_name, lat, lon) = if preset.name == "Auto" {
                resolve_auto_location().unwrap_or(("Riyadh".into(), 24.7136, 46.6753))
            } else {
                (preset.name.to_owned(), preset.lat, preset.lon)
            };

            if let Some(fetched) = fetch_open_meteo(&city_name, lat, lon)
                && let Ok(mut d) = data_arc.lock()
            {
                *d = fetched;
            }
            if let Ok(mut f) = fetching_arc.lock() {
                *f = false;
            }
        });
    }
}

fn resolve_auto_location() -> Option<(String, f64, f64)> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(4))
        .build();
    let val: serde_json::Value = agent
        .get("http://ip-api.com/json/?fields=status,city,lat,lon")
        .call()
        .ok()?
        .into_json()
        .ok()?;
    if val.get("status")?.as_str()? != "success" {
        return None;
    }
    let city = val.get("city")?.as_str()?.to_owned();
    let lat = val.get("lat")?.as_f64()?;
    let lon = val.get("lon")?.as_f64()?;
    Some((city, lat, lon))
}

fn fetch_open_meteo(city: &str, lat: f64, lon: f64) -> Option<WeatherData> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m&hourly=temperature_2m&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let root: serde_json::Value = agent.get(&url).call().ok()?.into_json().ok()?;
    let cur = root.get("current")?;
    let temp_c = cur.get("temperature_2m")?.as_f64()? as f32;
    let feels_c = cur
        .get("apparent_temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::from(temp_c)) as f32;
    let humidity = cur
        .get("relative_humidity_2m")
        .and_then(|v| v.as_u64())
        .unwrap_or(40) as u32;
    let wind_kmh = cur
        .get("wind_speed_10m")
        .and_then(|v| v.as_f64())
        .unwrap_or(8.0) as f32;
    let code = cur.get("weather_code").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let mut hourly = Vec::new();
    if let Some(arr) = root
        .get("hourly")
        .and_then(|h| h.get("temperature_2m"))
        .and_then(|a| a.as_array())
    {
        for v in arr.iter().take(24) {
            if let Some(t) = v.as_f64() {
                hourly.push(t as f32);
            }
        }
    }
    let (mut high_c, mut low_c) = (temp_c + 3.0, temp_c - 3.0);
    if let Some(daily) = root.get("daily") {
        if let Some(h) = daily
            .get("temperature_2m_max")
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_f64())
        {
            high_c = h as f32;
        }
        if let Some(l) = daily
            .get("temperature_2m_min")
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_f64())
        {
            low_c = l as f32;
        }
    }

    Some(WeatherData {
        city: city.to_owned(),
        temp_c,
        feels_c,
        high_c,
        low_c,
        humidity,
        wind_kmh,
        code,
        hourly,
        updated_at: Instant::now(),
    })
}

impl Module for Weather {
    fn id(&self) -> &'static str {
        "weather"
    }

    fn name(&self) -> &str {
        "Weather"
    }

    fn icon(&self) -> &'static str {
        icons::CLOUD_SUN
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        if self
            .last_fetch
            .is_none_or(|t| t.elapsed() >= REFRESH_INTERVAL)
        {
            self.trigger_fetch();
        }
        let snap = self
            .data
            .lock()
            .map(|d| d.clone())
            .unwrap_or_default();
        let (cond, cond_icon) = condition_label_and_icon(snap.code);
        let moon = current_moon_info();
        let content = ui.max_rect();
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, cond_icon, &snap.city, false, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &self.format_temp(snap.temp_c),
                    Theme::font_semibold(15.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &nook_ui::i18n::tr(cond),
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
            TileSize::Wide => {
                tile_header(ui, cond_icon, &snap.city, false, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &self.format_temp(snap.temp_c),
                    Theme::font_semibold(14.5),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.right(), y),
                    Align2::RIGHT_CENTER,
                    &nook_ui::i18n::tr(cond),
                    Theme::font_semibold(10.5),
                    theme.accent_fg(),
                );
            }
            TileSize::Large => {
                tile_header(ui, cond_icon, &snap.city, false, theme);
                let p = ui.painter();
                let top = content.top() + 22.0;
                kit::text(
                    p,
                    pos2(content.left(), top),
                    Align2::LEFT_TOP,
                    &self.format_temp(snap.temp_c),
                    Theme::font_semibold(24.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.right(), top + 4.0),
                    Align2::RIGHT_TOP,
                    &nook_ui::i18n::tr(cond),
                    Theme::font_semibold(SIZE_LABEL),
                    theme.accent_fg(),
                );
                kit::text(
                    p,
                    pos2(content.left(), top + 30.0),
                    Align2::LEFT_TOP,
                    &format!(
                        "H:{:.0}° L:{:.0}° · {}% · {:.0}km/h",
                        snap.high_c, snap.low_c, snap.humidity, snap.wind_kmh
                    ),
                    Theme::font(10.0),
                    theme.text_dim,
                );
                let chart_rect = Rect::from_min_max(
                    pos2(content.left(), top + 48.0),
                    pos2(content.right(), content.bottom() - 16.0),
                );
                draw_temp_sparkline(p, chart_rect, &snap.hourly, theme);
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!(
                        "{} ({:.0}%)",
                        nook_ui::i18n::tr(moon.phase_name),
                        moon.illumination
                    ),
                    Theme::font(10.0),
                    theme.text_faint,
                );
            }
        }

        if ui
            .interact(content.expand(8.0), ui.id().with("weather-open"), Sense::click())
            .clicked()
        {
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 265.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let snap = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let moon = current_moon_info();
        let mut trigger = false;

        let unit_label = if self.fahrenheit { "°F" } else { "°C" };
        if kit::pill(actions, unit_label, self.fahrenheit, theme).clicked() {
            self.fahrenheit = !self.fahrenheit;
        }
        actions.add_space(4.0);
        if widgets::icon_button(actions, icons::ARROWS_CLOCKWISE, 24.0, false, theme).clicked() {
            trigger = true;
        }

        let p = ui.painter();
        // City selector row
        let mut cx = body.left();
        let city_y = body.top();
        for (idx, city) in CITIES.iter().enumerate() {
            let active = self.selected_city == idx;
            let label = nook_ui::i18n::tr(city.name);
            let w = (label.chars().count() as f32 * 6.5 + 18.0).max(48.0);
            let r = Rect::from_min_size(pos2(cx, city_y), vec2(w, 22.0));
            if r.right() <= body.right() {
                let resp = ui.interact(r, ui.id().with(("w-city", idx)), Sense::click());
                let fill = if active {
                    with_alpha(theme.accent, 0.35)
                } else if resp.hovered() {
                    theme.tile_top
                } else {
                    with_alpha(theme.tile_bottom, 0.7)
                };
                p.rect_filled(r, 11.0, fill);
                kit::text(
                    p,
                    r.center(),
                    Align2::CENTER_CENTER,
                    &label,
                    Theme::font_semibold(10.5),
                    if active { theme.accent_fg() } else { theme.text_dim },
                );
                if resp.clicked() && self.selected_city != idx {
                    self.selected_city = idx;
                    trigger = true;
                }
                cx += w + 6.0;
            }
        }

        // Left hero card: Current weather
        let top_y = body.top() + 30.0;
        let left_card = Rect::from_min_size(pos2(body.left(), top_y), vec2(320.0, 92.0));
        let right_card = Rect::from_min_max(
            pos2(left_card.right() + 10.0, top_y),
            pos2(body.right(), top_y + 92.0),
        );
        kit::card(p, left_card, theme);
        kit::card(p, right_card, theme);

        let (cond, cond_icon) = condition_label_and_icon(snap.code);
        kit::text(
            p,
            pos2(left_card.left() + 14.0, left_card.top() + 12.0),
            Align2::LEFT_TOP,
            &format!("{cond_icon}  {}", nook_ui::i18n::tr(&snap.city)),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );
        kit::text(
            p,
            pos2(left_card.left() + 14.0, left_card.top() + 34.0),
            Align2::LEFT_TOP,
            &self.format_temp(snap.temp_c),
            Theme::font_semibold(30.0),
            theme.text,
        );
        kit::text(
            p,
            pos2(left_card.left() + 115.0, left_card.top() + 40.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr(cond),
            Theme::font_semibold(SIZE_BODY),
            theme.accent_fg(),
        );
        kit::text(
            p,
            pos2(left_card.left() + 115.0, left_card.top() + 60.0),
            Align2::LEFT_TOP,
            &format!(
                "{}: {}  ·  {}: {}%  ·  {:.0} km/h",
                nook_ui::i18n::tr("Feels like"),
                self.format_temp(snap.feels_c),
                nook_ui::i18n::tr("Humidity"),
                snap.humidity,
                snap.wind_kmh
            ),
            Theme::font(SIZE_LABEL),
            theme.text_dim,
        );

        // Right card: Lunar Phase & Sun/Moon telemetry
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 12.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::MOON_STARS, nook_ui::i18n::tr("Moon Phase")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 36.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr(moon.phase_name),
            Theme::font_semibold(15.0),
            theme.accent_fg(),
        );
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 58.0),
            Align2::LEFT_TOP,
            &format!(
                "{}: {:.0}%  ·  {}: {:.1}d",
                nook_ui::i18n::tr("Illumination"),
                moon.illumination,
                nook_ui::i18n::tr("Age"),
                moon.age_days
            ),
            Theme::font(SIZE_LABEL),
            theme.text_dim,
        );

        // Bottom card: 24-hour forecast sparkline
        let chart_card = Rect::from_min_max(
            pos2(body.left(), top_y + 100.0),
            pos2(body.right(), body.bottom()),
        );
        kit::card(p, chart_card, theme);
        kit::text(
            p,
            pos2(chart_card.left() + 12.0, chart_card.top() + 8.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("24-Hour Temperature Forecast"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );
        let plot_rect = Rect::from_min_max(
            pos2(chart_card.left() + 14.0, chart_card.top() + 28.0),
            pos2(chart_card.right() - 14.0, chart_card.bottom() - 20.0),
        );
        draw_temp_sparkline(p, plot_rect, &snap.hourly, theme);
        for (h_idx, label) in [(0usize, "00:00"), (6, "06:00"), (12, "12:00"), (18, "18:00"), (23, "23:00")] {
            let x = plot_rect.left() + (h_idx as f32 / 23.0) * plot_rect.width();
            kit::text(
                p,
                pos2(x, chart_card.bottom() - 4.0),
                Align2::CENTER_BOTTOM,
                label,
                Theme::font(10.0),
                theme.text_faint,
            );
        }

        if trigger {
            self.trigger_fetch();
        }
        false
    }
}

fn draw_temp_sparkline(p: &egui::Painter, rect: Rect, temps: &[f32], theme: &Theme) {
    if temps.len() < 2 || rect.height() < 8.0 {
        return;
    }
    let min_t = temps.iter().copied().fold(f32::INFINITY, f32::min);
    let max_t = temps.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (max_t - min_t).max(1.0);
    let n = temps.len() - 1;
    let pts: Vec<Pos2> = temps
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let x = rect.left() + (i as f32 / n as f32) * rect.width();
            let norm = (t - min_t) / span;
            let y = rect.bottom() - norm * rect.height();
            pos2(x, y)
        })
        .collect();
    for pair in pts.windows(2) {
        p.line_segment([pair[0], pair[1]], Stroke::new(2.0, theme.accent_fg()));
    }
    for &pt in pts.iter().step_by(4) {
        p.circle_filled(pt, 2.5, Color32::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moon_phase_at_reference_is_new_moon() {
        let info = moon_info_at(REF_NEW_MOON_UNIX);
        assert_eq!(info.phase_name, "New Moon");
        assert!(info.illumination < 1.0);
        let full = moon_info_at(REF_NEW_MOON_UNIX + (SYNODIC_DAYS * 0.5) * 86_400.0);
        assert_eq!(full.phase_name, "Full Moon");
        assert!(full.illumination > 99.0);
    }
}
