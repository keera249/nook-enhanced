//! Daily Screen Time & App Usage Tracker: monitors active foreground apps
//! throughout the day, persists daily usage to `%APPDATA%\Nook\screen_time.txt`,
//! and visualizes top used apps with usage bars (inspired by NotchPixel's
//! ScreenTimeTracker & AppTimeLog widget).

use crate::kit;
use egui::{Align2, Rect, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};
use nook_platform::{Hwnd, system};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::icons;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const SAVE_INTERVAL: Duration = Duration::from_secs(30);

pub struct ScreenTime {
    date_key: String,
    apps: HashMap<String, u64>,
    last_tick: Instant,
    last_save: Instant,
    current_app: String,
    open_page: bool,
}

impl Default for ScreenTime {
    fn default() -> Self {
        let today = today_key();
        let apps = load_screen_time(&today);
        Self {
            date_key: today,
            apps,
            last_tick: Instant::now(),
            last_save: Instant::now(),
            current_app: "Desktop".into(),
            open_page: false,
        }
    }
}

fn today_key() -> String {
    let lt = system::local_time();
    format!("{:04}-{:02}-{:02}", lt.year, lt.month, lt.day)
}

fn storage_path() -> Option<PathBuf> {
    nook_platform::files::config_dir().map(|d| d.join("screen_time.txt"))
}

fn load_screen_time(expected_date: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    let Some(path) = storage_path() else {
        return map;
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return map;
    };
    let mut lines = raw.lines();
    let Some(header) = lines.next() else {
        return map;
    };
    if header.trim() != expected_date {
        return map;
    }
    for line in lines {
        if let Some((name, secs_str)) = line.rsplit_once('\t')
            && let Ok(secs) = secs_str.trim().parse::<u64>()
            && !name.trim().is_empty()
        {
            map.insert(name.trim().to_owned(), secs);
        }
    }
    map
}

fn save_screen_time(date_key: &str, apps: &HashMap<String, u64>) {
    let Some(path) = storage_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = format!("{date_key}\n");
    for (name, secs) in apps {
        out.push_str(&format!("{name}\t{secs}\n"));
    }
    let _ = std::fs::write(path, out);
}

pub fn format_duration_hm(total_secs: u64) -> String {
    let hrs = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    if hrs > 0 {
        format!("{hrs}h {mins:02}m")
    } else if mins > 0 {
        format!("{mins}m {}s", total_secs % 60)
    } else {
        format!("{}s", total_secs)
    }
}

impl ScreenTime {
    pub fn new() -> Self {
        Self::default()
    }

    fn sample(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        if elapsed < SAMPLE_INTERVAL {
            return;
        }
        self.last_tick = now;
        let today = today_key();
        if self.date_key != today {
            self.date_key = today;
            self.apps.clear();
        }
        let add_secs = elapsed.as_secs().clamp(1, 15);
        if let Some(app) = system::foreground_app_name(Hwnd::from_raw(0)) {
            self.current_app = app.clone();
            *self.apps.entry(app).or_insert(0) += add_secs;
        }
        if now.duration_since(self.last_save) >= SAVE_INTERVAL {
            self.last_save = now;
            save_screen_time(&self.date_key, &self.apps);
        }
    }

    fn sorted_apps(&self) -> Vec<(String, u64)> {
        let mut list: Vec<(String, u64)> =
            self.apps.iter().map(|(k, &v)| (k.clone(), v)).collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.1));
        list
    }

    fn total_secs(&self) -> u64 {
        self.apps.values().copied().sum::<u64>().max(1)
    }
}

impl Module for ScreenTime {
    fn id(&self) -> &'static str {
        "screen_time"
    }

    fn name(&self) -> &str {
        "Screen Time"
    }

    fn icon(&self) -> &'static str {
        icons::CHART_BAR_HORIZONTAL
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn next_repaint(&self, _tile_visible: bool) -> Option<Duration> {
        Some(SAMPLE_INTERVAL)
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        self.sample();
        let total = self.total_secs();
        let sorted = self.sorted_apps();
        let top_app_raw = sorted
            .first()
            .map(|(n, _)| n.as_str())
            .unwrap_or(self.current_app.as_str());
        let top_app_str = nook_ui::i18n::tr(top_app_raw);
        let top_app = top_app_str.as_str();
        let fmt_dur = |s: u64| -> String {
            if nook_ui::i18n::is_arabic() {
                let h = s / 3600;
                let m = (s % 3600) / 60;
                let sec = s % 60;
                if h > 0 {
                    format!("{h}س {m:02}د")
                } else if m > 0 {
                    format!("{m}د {sec}ث")
                } else {
                    format!("{sec}ث")
                }
            } else {
                format_duration_hm(s)
            }
        };
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::CHART_BAR_HORIZONTAL, "Screen Time", false, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &fmt_dur(total),
                    Theme::font_semibold(14.5),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    top_app,
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::CHART_BAR_HORIZONTAL, "Screen Time", false, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &fmt_dur(total),
                    Theme::font_semibold(13.5),
                    theme.text,
                );
                let app_short: String = top_app.chars().take(9).collect();
                kit::text(
                    p,
                    pos2(content.right(), y),
                    Align2::RIGHT_CENTER,
                    &app_short,
                    Theme::font_semibold(10.0),
                    theme.accent_fg(),
                );
            }
            TileSize::Large => {
                tile_header(ui, icons::CHART_BAR_HORIZONTAL, "Screen Time", false, theme);
                let p = ui.painter();
                let top = content.top() + 21.0;
                kit::text(
                    p,
                    pos2(content.left(), top),
                    Align2::LEFT_TOP,
                    &fmt_dur(total),
                    Theme::font_semibold(20.0),
                    theme.text,
                );
                for (i, (name, secs)) in sorted.iter().take(3).enumerate() {
                    let ry = top + 28.0 + i as f32 * 22.0;
                    let share = (*secs as f32 / total as f32).clamp(0.05, 1.0);
                    kit::text(
                        p,
                        pos2(content.left(), ry),
                        Align2::LEFT_TOP,
                        &nook_ui::i18n::tr(name),
                        Theme::font(10.0),
                        theme.text,
                    );
                    kit::text(
                        p,
                        pos2(content.right(), ry),
                        Align2::RIGHT_TOP,
                        &fmt_dur(*secs),
                        Theme::font(9.5),
                        theme.text_dim,
                    );
                    let br = Rect::from_min_size(
                        pos2(content.left(), ry + 13.0),
                        vec2(content.width(), 5.0),
                    );
                    kit::bar(p, br, share, theme.accent_fg(), theme);
                }
            }
        }

        if ui
            .interact(rect, ui.id().with("st-open"), Sense::click())
            .clicked()
        {
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 260.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        self.sample();
        let total = self.total_secs();
        let sorted = self.sorted_apps();
        let mut reset_today = false;
        let fmt_dur = |s: u64| -> String {
            if nook_ui::i18n::is_arabic() {
                let h = s / 3600;
                let m = (s % 3600) / 60;
                let sec = s % 60;
                if h > 0 {
                    format!("{h}س {m:02}د")
                } else if m > 0 {
                    format!("{m}د {sec}ث")
                } else {
                    format!("{sec}ث")
                }
            } else {
                format_duration_hm(s)
            }
        };

        if kit::pill(actions, "Reset Today", false, theme).clicked() {
            reset_today = true;
        }

        let p = ui.painter();
        let summary_r =
            Rect::from_min_size(pos2(body.left(), body.top()), vec2(190.0, body.height()));
        let list_card = Rect::from_min_max(pos2(summary_r.right() + 10.0, body.top()), body.max);
        kit::card(p, summary_r, theme);
        kit::card(p, list_card, theme);

        // Summary card
        kit::text(
            p,
            pos2(summary_r.left() + 14.0, summary_r.top() + 14.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("Today's Total"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );
        kit::text(
            p,
            pos2(summary_r.left() + 14.0, summary_r.top() + 34.0),
            Align2::LEFT_TOP,
            &fmt_dur(total),
            Theme::font_semibold(28.0),
            theme.text,
        );
        kit::text(
            p,
            pos2(summary_r.left() + 14.0, summary_r.top() + 78.0),
            Align2::LEFT_TOP,
            &format!("{}: {}", nook_ui::i18n::tr("Apps Tracked"), sorted.len()),
            Theme::font(SIZE_LABEL),
            theme.accent_fg(),
        );
        kit::text(
            p,
            pos2(summary_r.left() + 14.0, summary_r.top() + 102.0),
            Align2::LEFT_TOP,
            &format!("{}:", nook_ui::i18n::tr("Active Now")),
            Theme::font(SIZE_LABEL),
            theme.text_faint,
        );
        kit::text(
            p,
            pos2(summary_r.left() + 14.0, summary_r.top() + 120.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr(&self.current_app),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        // Ranked app usage bars
        kit::text(
            p,
            pos2(list_card.left() + 12.0, list_card.top() + 10.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("Top Applications Today"),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        let inner = Rect::from_min_max(
            pos2(list_card.left() + 12.0, list_card.top() + 34.0),
            pos2(list_card.right() - 12.0, list_card.bottom() - 10.0),
        );
        let mut sui = ui.new_child(UiBuilder::new().max_rect(inner));
        ScrollArea::vertical()
            .id_salt("st-scroll")
            .max_height(inner.height())
            .show(&mut sui, |ui| {
                for (name, secs) in &sorted {
                    let (row, _) =
                        ui.allocate_exact_size(vec2(inner.width() - 6.0, 32.0), Sense::hover());
                    let share = (*secs as f32 / total as f32).clamp(0.02, 1.0);
                    let p = ui.painter();
                    kit::text(
                        p,
                        pos2(row.left(), row.top() + 2.0),
                        Align2::LEFT_TOP,
                        &nook_ui::i18n::tr(name),
                        Theme::font_semibold(11.5),
                        theme.text,
                    );
                    kit::text(
                        p,
                        pos2(row.right(), row.top() + 2.0),
                        Align2::RIGHT_TOP,
                        &format!("{} ({:.0}%)", fmt_dur(*secs), share * 100.0),
                        Theme::font(11.0),
                        theme.text_dim,
                    );
                    let br = Rect::from_min_size(
                        pos2(row.left(), row.top() + 20.0),
                        vec2(row.width(), 7.0),
                    );
                    kit::bar(p, br, share, with_alpha(theme.accent_fg(), 0.9), theme);
                }
            });

        if reset_today {
            self.apps.clear();
            save_screen_time(&self.date_key, &self.apps);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_formatting_works() {
        assert_eq!(format_duration_hm(45), "45s");
        assert_eq!(format_duration_hm(125), "2m 5s");
        assert_eq!(format_duration_hm(3660), "1h 01m");
    }
}
