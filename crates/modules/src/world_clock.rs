//! World Clock, Time Progress & Event Countdown: multi-city world clocks with
//! day/night indicators, live Day/Week/Month/Year progress bars, and custom
//! event countdown (inspired by NotchPixel's WorldClock, TimeProgress, and
//! Countdown widgets).

use crate::kit;
use egui::{Align2, Rect, Sense, Ui, pos2, vec2};
use nook_platform::system;
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::icons;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
struct ZonePreset {
    city: &'static str,
    offset_mins: i32,
}

const ZONES: &[ZonePreset] = &[
    ZonePreset {
        city: "Makkah",
        offset_mins: 180,
    },
    ZonePreset {
        city: "Cairo",
        offset_mins: 120,
    },
    ZonePreset {
        city: "Dubai",
        offset_mins: 240,
    },
    ZonePreset {
        city: "London",
        offset_mins: 0,
    },
    ZonePreset {
        city: "New York",
        offset_mins: -240,
    },
    ZonePreset {
        city: "Tokyo",
        offset_mins: 540,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct TimeProgressStats {
    pub day_pct: f32,
    pub week_pct: f32,
    pub month_pct: f32,
    pub year_pct: f32,
    pub day_of_year: u32,
    pub days_in_year: u32,
}

pub fn compute_time_progress(lt: system::LocalTime) -> TimeProgressStats {
    let secs_in_day = lt.hour * 3600 + lt.minute * 60 + lt.second;
    let day_pct = (secs_in_day as f32 / 86_400.0).clamp(0.0, 1.0);

    let leap = (lt.year % 4 == 0 && lt.year % 100 != 0) || (lt.year % 400 == 0);
    let month_lengths: [u32; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let m_idx = (lt.month.saturating_sub(1) as usize).min(11);
    let days_in_month = month_lengths[m_idx].max(1);
    let month_pct =
        (((lt.day.saturating_sub(1)) as f32 + day_pct) / days_in_month as f32).clamp(0.0, 1.0);

    let prev_days: u32 = month_lengths[..m_idx].iter().sum();
    let day_of_year = prev_days + lt.day;
    let days_in_year = if leap { 366 } else { 365 };
    let year_pct =
        (((day_of_year.saturating_sub(1)) as f32 + day_pct) / days_in_year as f32).clamp(0.0, 1.0);

    // Sakamoto's weekday (0 = Sun .. 6 = Sat)
    let weekday = weekday_index(lt.year, lt.month, lt.day);
    let week_pct = ((weekday as f32 + day_pct) / 7.0).clamp(0.0, 1.0);

    TimeProgressStats {
        day_pct,
        week_pct,
        month_pct,
        year_pct,
        day_of_year,
        days_in_year,
    }
}

fn weekday_index(y: i32, m: u32, d: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y_adj = if m < 3 { y - 1 } else { y };
    let w = (y_adj + y_adj / 4 - y_adj / 100 + y_adj / 400 + t[(m.saturating_sub(1) as usize).min(11)] + d as i32)
        .rem_euclid(7);
    w as u32
}

fn zone_time(offset_mins: i32) -> (u32, u32, bool) {
    let utc_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let shifted = (utc_secs + i64::from(offset_mins) * 60).rem_euclid(86_400);
    let hour = (shifted / 3600) as u32;
    let min = ((shifted % 3600) / 60) as u32;
    let is_day = (6..18).contains(&hour);
    (hour, min, is_day)
}

#[derive(Default)]
pub struct WorldClock {
    selected_event: usize,
    open_page: bool,
}

impl WorldClock {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Module for WorldClock {
    fn id(&self) -> &'static str {
        "world_clock"
    }

    fn name(&self) -> &str {
        "World Clock & Year"
    }

    fn icon(&self) -> &'static str {
        icons::GLOBE_HEMISPHERE_WEST
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn next_repaint(&self, _tile_visible: bool) -> Option<Duration> {
        Some(Duration::from_secs(15))
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        let lt = system::local_time();
        let prog = compute_time_progress(lt);
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::GLOBE_HEMISPHERE_WEST, "Time", false, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &format!("{:.0}%", prog.day_pct * 100.0),
                    Theme::font_semibold(15.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!("{}: {:.0}%", nook_ui::i18n::tr("Year"), prog.year_pct * 100.0),
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::GLOBE_HEMISPHERE_WEST, "World Clock", false, theme);
                let p = ui.painter();
                let (mh, mm, _) = zone_time(180);
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &format!("{}: {:02}:{:02}", nook_ui::i18n::tr("Makkah"), mh, mm),
                    Theme::font_semibold(11.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.right(), y),
                    Align2::RIGHT_CENTER,
                    &format!("{:.0}%", prog.year_pct * 100.0),
                    Theme::font_semibold(10.5),
                    theme.accent_fg(),
                );
            }
            TileSize::Large => {
                tile_header(ui, icons::GLOBE_HEMISPHERE_WEST, "World Clock", false, theme);
                let p = ui.painter();
                let top = content.top() + 21.0;

                // 3 compact city clocks
                for (i, z) in [ZONES[0], ZONES[3], ZONES[5]].iter().enumerate() {
                    let (h, m, day) = zone_time(z.offset_mins);
                    let ry = top + i as f32 * 18.0;
                    let ic = if day { icons::SUN } else { icons::MOON };
                    kit::text(
                        p,
                        pos2(content.left(), ry),
                        Align2::LEFT_TOP,
                        &format!("{ic} {}", nook_ui::i18n::tr(z.city)),
                        Theme::font(10.5),
                        theme.text_dim,
                    );
                    kit::text(
                        p,
                        pos2(content.right(), ry),
                        Align2::RIGHT_TOP,
                        &format!("{:02}:{:02}", h, m),
                        Theme::font_semibold(11.0),
                        theme.text,
                    );
                }

                // Day & Year progress bars
                let py = top + 58.0;
                kit::text(
                    p,
                    pos2(content.left(), py),
                    Align2::LEFT_TOP,
                    &format!(
                        "{}: {:.0}%  ·  {}: {:.0}%",
                        nook_ui::i18n::tr("Day"),
                        prog.day_pct * 100.0,
                        nook_ui::i18n::tr("Year"),
                        prog.year_pct * 100.0
                    ),
                    Theme::font(9.5),
                    theme.text_dim,
                );
                let d_bar = Rect::from_min_size(
                    pos2(content.left(), py + 14.0),
                    vec2(content.width(), 5.0),
                );
                kit::bar(p, d_bar, prog.day_pct, theme.accent_fg(), theme);
                let y_bar = Rect::from_min_size(
                    pos2(content.left(), py + 23.0),
                    vec2(content.width(), 5.0),
                );
                kit::bar(
                    p,
                    y_bar,
                    prog.year_pct,
                    with_alpha(theme.accent_fg(), 0.65),
                    theme,
                );
            }
        }

        if ui
            .interact(rect, ui.id().with("wc-open"), Sense::click())
            .clicked()
        {
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 270.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, _actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let lt = system::local_time();
        let prog = compute_time_progress(lt);

        let p = ui.painter();
        let left_card =
            Rect::from_min_size(pos2(body.left(), body.top()), vec2(275.0, body.height()));
        let right_card = Rect::from_min_max(pos2(left_card.right() + 10.0, body.top()), body.max);
        kit::card(p, left_card, theme);
        kit::card(p, right_card, theme);

        // Left card: 6 World Cities
        kit::text(
            p,
            pos2(left_card.left() + 12.0, left_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::GLOBE, nook_ui::i18n::tr("Global Time Zones")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        for (i, z) in ZONES.iter().enumerate() {
            let (h, m, is_day) = zone_time(z.offset_mins);
            let ry = left_card.top() + 36.0 + i as f32 * 28.0;
            let row = Rect::from_min_size(
                pos2(left_card.left() + 10.0, ry),
                vec2(left_card.width() - 20.0, 24.0),
            );
            p.rect_filled(row, 6.0, theme.tile_bottom);
            let ic = if is_day { icons::SUN } else { icons::MOON };
            let sign = if z.offset_mins >= 0 { "+" } else { "" };
            let off_h = z.offset_mins / 60;
            kit::text(
                p,
                pos2(row.left() + 8.0, row.center().y),
                Align2::LEFT_CENTER,
                &format!("{ic}  {} (UTC{sign}{off_h})", nook_ui::i18n::tr(z.city)),
                Theme::font(11.5),
                theme.text,
            );
            kit::text(
                p,
                pos2(row.right() - 8.0, row.center().y),
                Align2::RIGHT_CENTER,
                &format!("{:02}:{:02}", h, m),
                Theme::font_semibold(12.5),
                theme.accent_fg(),
            );
        }

        // Right card: Time Progress (Day, Week, Month, Year) + Countdown
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::HOURGLASS_MEDIUM, nook_ui::i18n::tr("Time Progress")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        let bars = [
            ("Today", prog.day_pct),
            ("This Week", prog.week_pct),
            ("This Month", prog.month_pct),
            ("This Year", prog.year_pct),
        ];
        for (i, &(lbl, val)) in bars.iter().enumerate() {
            let by = right_card.top() + 36.0 + i as f32 * 32.0;
            kit::text(
                p,
                pos2(right_card.left() + 12.0, by),
                Align2::LEFT_TOP,
                &nook_ui::i18n::tr(lbl),
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
            kit::text(
                p,
                pos2(right_card.right() - 12.0, by),
                Align2::RIGHT_TOP,
                &format!("{:.1}%", val * 100.0),
                Theme::font_semibold(SIZE_LABEL),
                theme.text,
            );
            let br = Rect::from_min_size(
                pos2(right_card.left() + 12.0, by + 15.0),
                vec2(right_card.width() - 24.0, 8.0),
            );
            kit::bar(p, br, val, theme.accent_fg(), theme);
        }

        // Event Countdown Footer inside Right card
        let days_left_year = prog.days_in_year.saturating_sub(prog.day_of_year);
        let hours_left_day = 23u32.saturating_sub(lt.hour);
        let ev_r = Rect::from_min_max(
            pos2(right_card.left() + 12.0, right_card.bottom() - 42.0),
            pos2(right_card.right() - 12.0, right_card.bottom() - 10.0),
        );
        let ev_resp = ui.interact(ev_r, ui.id().with("wc-ev"), Sense::click());
        let p = ui.painter();
        p.rect_filled(ev_r, 8.0, with_alpha(theme.accent, 0.24));
        let msg = if self.selected_event % 2 == 0 {
            format!(
                "{}  {}: {}d {}h {}",
                icons::CALENDAR_CHECK,
                nook_ui::i18n::tr("New Year"),
                days_left_year,
                hours_left_day,
                nook_ui::i18n::tr("remaining")
            )
        } else {
            format!(
                "{}  {}: {}/{} {}",
                icons::CALENDAR_CHECK,
                nook_ui::i18n::tr("Day of Year"),
                prog.day_of_year,
                prog.days_in_year,
                nook_ui::i18n::tr("elapsed")
            )
        };
        kit::text(
            p,
            ev_r.center(),
            Align2::CENTER_CENTER,
            &msg,
            Theme::font_semibold(11.5),
            theme.text,
        );
        if ev_resp.clicked() {
            self.selected_event = self.selected_event.wrapping_add(1);
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_progress_bounds() {
        let lt = system::LocalTime {
            year: 2026,
            month: 6,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0,
        };
        let stats = compute_time_progress(lt);
        assert!((stats.day_pct - 0.5).abs() < 0.01);
        assert!(stats.year_pct > 0.4 && stats.year_pct < 0.5);
    }
}
