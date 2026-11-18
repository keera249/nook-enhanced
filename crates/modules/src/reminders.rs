//! Scheduled Reminders: time-based alerts (`HH:MM` or quick `+5m / +15m / +30m / +1h`)
//! persisted to `%APPDATA%\Nook\reminders.txt` that sound an alert and display
//! a badge in the collapsed Notch pill when due (inspired by NotchPixel's
//! ReminderList widget).

use crate::kit;
use egui::{Align2, Color32, Key, Rect, ScrollArea, Sense, TextEdit, Ui, UiBuilder, pos2, vec2};
use nook_platform::system;
use nook_ui::bento::tile_header;
use nook_ui::module::{Badge, Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, Theme, with_alpha};
use nook_ui::icons;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ReminderItem {
    pub hour: u32,
    pub minute: u32,
    pub title: String,
    pub fired_today: Option<u32>,
}

impl ReminderItem {
    pub fn time_str(&self) -> String {
        format!("{:02}:{:02}", self.hour % 24, self.minute % 60)
    }
}

pub struct Reminders {
    items: Vec<ReminderItem>,
    draft_title: String,
    draft_time: String,
    active_alert: Option<(String, Instant)>,
    open_page: bool,
}

impl Default for Reminders {
    fn default() -> Self {
        Self {
            items: load_reminders(),
            draft_title: String::new(),
            draft_time: "18:00".into(),
            active_alert: None,
            open_page: false,
        }
    }
}

fn reminders_path() -> Option<PathBuf> {
    nook_platform::files::config_dir().map(|d| d.join("reminders.txt"))
}

fn load_reminders() -> Vec<ReminderItem> {
    let Some(path) = reminders_path() else {
        return default_reminders();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return default_reminders();
    };
    let list: Vec<ReminderItem> = raw
        .lines()
        .filter_map(|line| {
            let (hm, title) = line.split_once('\t')?;
            let (h, m) = hm.trim().split_once(':')?;
            Some(ReminderItem {
                hour: h.parse::<u32>().ok()? % 24,
                minute: m.parse::<u32>().ok()? % 60,
                title: title.trim().to_owned(),
                fired_today: None,
            })
        })
        .collect();
    if list.is_empty() {
        default_reminders()
    } else {
        list
    }
}

fn default_reminders() -> Vec<ReminderItem> {
    vec![
        ReminderItem {
            hour: 14,
            minute: 0,
            title: "Drink water & stretch".into(),
            fired_today: None,
        },
        ReminderItem {
            hour: 20,
            minute: 0,
            title: "Evening wrap-up".into(),
            fired_today: None,
        },
    ]
}

fn save_reminders(items: &[ReminderItem]) {
    let Some(path) = reminders_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = String::new();
    for r in items {
        out.push_str(&format!("{:02}:{:02}\t{}\n", r.hour, r.minute, r.title));
    }
    let _ = std::fs::write(path, out);
}

impl Reminders {
    pub fn new() -> Self {
        Self::default()
    }

    fn check_due(&mut self) {
        if let Some((_, since)) = &self.active_alert
            && since.elapsed() > Duration::from_secs(45)
        {
            self.active_alert = None;
        }
        let lt = system::local_time();
        let day_stamp = lt.year as u32 * 10000 + lt.month * 100 + lt.day;
        let mut fired_title = None;
        for item in &mut self.items {
            if item.hour == lt.hour && item.minute == lt.minute && item.fired_today != Some(day_stamp)
            {
                item.fired_today = Some(day_stamp);
                fired_title = Some(item.title.clone());
            }
        }
        if let Some(title) = fired_title {
            system::alert_sound();
            self.active_alert = Some((title, Instant::now()));
        }
    }

    fn add_offset_mins(&mut self, add_mins: u32) {
        let lt = system::local_time();
        let total = (lt.hour * 60 + lt.minute + add_mins) % (24 * 60);
        let h = total / 60;
        let m = total % 60;
        let title = if self.draft_title.trim().is_empty() {
            format!("Reminder (+{add_mins}m)")
        } else {
            std::mem::take(&mut self.draft_title)
        };
        self.items.push(ReminderItem {
            hour: h,
            minute: m,
            title,
            fired_today: None,
        });
        self.items.sort_by_key(|r| (r.hour, r.minute));
        save_reminders(&self.items);
    }

    fn next_upcoming(&self) -> Option<&ReminderItem> {
        let lt = system::local_time();
        let now_m = lt.hour * 60 + lt.minute;
        self.items
            .iter()
            .find(|r| r.hour * 60 + r.minute >= now_m)
            .or_else(|| self.items.first())
    }
}

impl Module for Reminders {
    fn id(&self) -> &'static str {
        "reminders"
    }

    fn name(&self) -> &str {
        "Reminders"
    }

    fn icon(&self) -> &'static str {
        icons::BELL_RINGING
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn is_active(&self) -> bool {
        self.active_alert.is_some()
    }

    fn collapsed_badge(&self) -> Option<Badge> {
        self.active_alert.as_ref().map(|(title, _)| Badge::Ring {
            label: title.chars().take(12).collect(),
            progress: 1.0,
        })
    }

    fn next_repaint(&self, _tile_visible: bool) -> Option<Duration> {
        Some(Duration::from_secs(5))
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        self.check_due();
        let alerting = self.active_alert.is_some();
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::BELL_RINGING, "Alerts", alerting, theme);
                let p = ui.painter();
                let hm = self
                    .next_upcoming()
                    .map(|r| r.time_str())
                    .unwrap_or_else(|| "—".into());
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &hm,
                    Theme::font_semibold(15.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!("{} {}", self.items.len(), nook_ui::i18n::tr("set")),
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::BELL_RINGING, "Reminders", alerting, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                if let Some((alert_txt, _)) = &self.active_alert {
                    kit::one_line(
                        ui,
                        pos2(content.left(), content.top() + 21.0),
                        &format!("{} {}", icons::BELL_RINGING, alert_txt),
                        content.width(),
                        Theme::font_semibold(11.5),
                        theme.accent_fg(),
                    );
                } else if let Some(next) = self.next_upcoming() {
                    kit::text(
                        p,
                        pos2(content.left(), y),
                        Align2::LEFT_CENTER,
                        &next.time_str(),
                        Theme::font_semibold(13.5),
                        theme.text,
                    );
                    let short_title: String = nook_ui::i18n::tr(&next.title).chars().take(10).collect();
                    kit::text(
                        p,
                        pos2(content.right(), y),
                        Align2::RIGHT_CENTER,
                        &short_title,
                        Theme::font(10.0),
                        theme.text_dim,
                    );
                } else {
                    kit::text(
                        p,
                        pos2(content.left(), y),
                        Align2::LEFT_CENTER,
                        &nook_ui::i18n::tr("No reminders set"),
                        Theme::font(10.5),
                        theme.text_dim,
                    );
                }
            }
            TileSize::Large => {
                tile_header(ui, icons::BELL_RINGING, "Reminders", alerting, theme);
                let p = ui.painter();
                let top = content.top() + 22.0;
                for (i, r) in self.items.iter().take(4).enumerate() {
                    let ry = top + i as f32 * 20.0;
                    kit::text(
                        p,
                        pos2(content.left(), ry),
                        Align2::LEFT_TOP,
                        &r.time_str(),
                        Theme::font_semibold(11.0),
                        theme.accent_fg(),
                    );
                    kit::text(
                        p,
                        pos2(content.left() + 42.0, ry),
                        Align2::LEFT_TOP,
                        &nook_ui::i18n::tr(&r.title),
                        Theme::font(10.5),
                        theme.text,
                    );
                }
            }
        }

        if ui
            .interact(rect, ui.id().with("rem-open"), Sense::click())
            .clicked()
        {
            if self.active_alert.is_some() {
                self.active_alert = None;
            }
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 255.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        self.check_due();
        let mut quick_mins = None;

        for &mins in &[60u32, 30, 15, 5] {
            if kit::pill(actions, &format!("+{mins}m"), false, theme).clicked() {
                quick_mins = Some(mins);
            }
            actions.add_space(4.0);
        }

        let p = ui.painter();
        let card_r = body;
        kit::card(p, card_r, theme);

        // Top input bar: HH:MM + Title + Add button
        let time_r = Rect::from_min_size(
            pos2(card_r.left() + 12.0, card_r.top() + 12.0),
            vec2(64.0, 28.0),
        );
        let title_r = Rect::from_min_size(
            pos2(time_r.right() + 8.0, card_r.top() + 12.0),
            vec2(card_r.width() - 134.0, 28.0),
        );
        let add_r = Rect::from_min_size(
            pos2(title_r.right() + 8.0, card_r.top() + 12.0),
            vec2(34.0, 28.0),
        );
        p.rect_filled(time_r, 6.0, theme.tile_bottom);
        p.rect_filled(title_r, 6.0, theme.tile_bottom);

        let mut tui = ui.new_child(UiBuilder::new().max_rect(time_r.shrink2(vec2(6.0, 4.0))));
        tui.add(
            TextEdit::singleline(&mut self.draft_time)
                .hint_text("18:00")
                .frame(egui::Frame::NONE)
                .font(Theme::font_semibold(SIZE_BODY)),
        );

        let mut m_ui = ui.new_child(UiBuilder::new().max_rect(title_r.shrink2(vec2(8.0, 4.0))));
        let edit = m_ui.add(
            TextEdit::singleline(&mut self.draft_title)
                .hint_text(nook_ui::i18n::tr("Reminder note…"))
                .frame(egui::Frame::NONE)
                .font(Theme::font(SIZE_BODY)),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let add_resp = ui.interact(add_r, ui.id().with("rem-add"), Sense::click());
        ui.painter()
            .rect_filled(add_r, 6.0, with_alpha(theme.accent, 0.38));
        kit::text(
            ui.painter(),
            add_r.center(),
            Align2::CENTER_CENTER,
            icons::PLUS,
            Theme::font_semibold(14.0),
            theme.accent_fg(),
        );

        if (enter || add_resp.clicked()) && !self.draft_title.trim().is_empty() {
            let (h, m) = self
                .draft_time
                .trim()
                .split_once(':')
                .and_then(|(a, b)| Some((a.parse::<u32>().ok()? % 24, b.parse::<u32>().ok()? % 60)))
                .unwrap_or((18, 0));
            self.items.push(ReminderItem {
                hour: h,
                minute: m,
                title: self.draft_title.trim().to_owned(),
                fired_today: None,
            });
            self.items.sort_by_key(|r| (r.hour, r.minute));
            self.draft_title.clear();
            save_reminders(&self.items);
        }

        // List of reminders
        let list_r = Rect::from_min_max(
            pos2(card_r.left() + 12.0, card_r.top() + 48.0),
            pos2(card_r.right() - 12.0, card_r.bottom() - 10.0),
        );
        let mut lui = ui.new_child(UiBuilder::new().max_rect(list_r));
        let mut del_idx = None;
        ScrollArea::vertical()
            .id_salt("rem-scroll")
            .max_height(list_r.height())
            .show(&mut lui, |ui| {
                for (idx, item) in self.items.iter().enumerate() {
                    let (row, _) =
                        ui.allocate_exact_size(vec2(list_r.width() - 6.0, 28.0), Sense::hover());
                    let del_r = Rect::from_min_size(
                        pos2(row.right() - 24.0, row.top() + 4.0),
                        vec2(20.0, 20.0),
                    );
                    let del_resp =
                        ui.interact(del_r, ui.id().with(("rem-del", idx)), Sense::click());
                    let p = ui.painter();
                    p.rect_filled(row, 6.0, theme.tile_bottom);
                    kit::text(
                        p,
                        pos2(row.left() + 10.0, row.center().y),
                        Align2::LEFT_CENTER,
                        &format!("{}  {}", icons::CLOCK, item.time_str()),
                        Theme::font_semibold(12.0),
                        theme.accent_fg(),
                    );
                    kit::text(
                        p,
                        pos2(row.left() + 78.0, row.center().y),
                        Align2::LEFT_CENTER,
                        &item.title,
                        Theme::font(SIZE_BODY),
                        theme.text,
                    );
                    kit::text(
                        p,
                        del_r.center(),
                        Align2::CENTER_CENTER,
                        icons::TRASH,
                        Theme::font(12.0),
                        if del_resp.hovered() {
                            Color32::from_rgb(245, 95, 95)
                        } else {
                            theme.text_faint
                        },
                    );
                    if del_resp.clicked() {
                        del_idx = Some(idx);
                    }
                }
            });

        if let Some(i) = del_idx
            && i < self.items.len()
        {
            self.items.remove(i);
            save_reminders(&self.items);
        }

        if let Some(m) = quick_mins {
            self.add_offset_mins(m);
        }
        false
    }
}
