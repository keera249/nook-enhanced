//! Hydration & Daily Habits Tracker: daily water intake counter (cups/ml) and
//! daily habit checklist with streak tracking persisted to `%APPDATA%\Nook\habits.txt`
//! (inspired by NotchPixel's WaterReminderWidget & HabitTrackerWidget).

use crate::kit;
use egui::{Align2, Color32, Id, Rect, Sense, Ui, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::{icons, paint};
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};

#[derive(Clone, Debug)]
struct Habit {
    title: String,
    done: bool,
    streak: u32,
}

pub struct HabitWater {
    day_key: u64,
    cups: u32,
    goal_cups: u32,
    habits: Vec<Habit>,
    new_habit: String,
    open_page: bool,
}

impl Default for HabitWater {
    fn default() -> Self {
        let mut s = Self {
            day_key: current_day_key(),
            cups: 0,
            goal_cups: 8,
            habits: Vec::new(),
            new_habit: String::new(),
            open_page: false,
        };
        s.load();
        if s.habits.is_empty() {
            s.habits = vec![
                Habit {
                    title: "شرب ٨ أكواب ماء (٢ لتر)".into(),
                    done: false,
                    streak: 3,
                },
                Habit {
                    title: "المشي أو الرياضة ٣٠ دقيقة".into(),
                    done: false,
                    streak: 5,
                },
                Habit {
                    title: "قراءة أو تعلم ٢٠ دقيقة".into(),
                    done: false,
                    streak: 2,
                },
                Habit {
                    title: "إنجاز أهم ٣ مهام لليوم".into(),
                    done: false,
                    streak: 4,
                },
            ];
            s.save();
        }
        s
    }
}

fn current_day_key() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0)
}

impl HabitWater {
    fn storage_path() -> Option<std::path::PathBuf> {
        let dir = nook_platform::files::config_dir()?;
        let _ = std::fs::create_dir_all(&dir);
        Some(dir.join("habits.txt"))
    }

    fn load(&mut self) {
        let Some(p) = Self::storage_path() else {
            return;
        };
        let Ok(txt) = std::fs::read_to_string(p) else {
            return;
        };
        let today = current_day_key();
        let mut saved_day = today;
        let mut habits = Vec::new();
        for (idx, line) in txt.lines().enumerate() {
            if idx == 0 && let Some(rest) = line.strip_prefix("WATER:") {
                let parts: Vec<&str> = rest.split('|').collect();
                if parts.len() >= 3 {
                    saved_day = parts[0].parse().unwrap_or(today);
                    self.cups = parts[1].parse().unwrap_or(0);
                    self.goal_cups = parts[2].parse().unwrap_or(8).clamp(4, 16);
                }
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                let done = parts[0] == "1";
                let streak = parts[1].parse().unwrap_or(0);
                let title = parts[2].trim().to_owned();
                if !title.is_empty() {
                    habits.push(Habit {
                        title,
                        done,
                        streak,
                    });
                }
            }
        }
        if saved_day != today {
            self.day_key = today;
            self.cups = 0;
            for h in &mut habits {
                if h.done {
                    h.streak = h.streak.saturating_add(1);
                }
                h.done = false;
            }
        }
        if !habits.is_empty() {
            self.habits = habits;
        }
    }

    fn save(&self) {
        let Some(p) = Self::storage_path() else {
            return;
        };
        let mut out = format!("WATER:{}|{}|{}\n", self.day_key, self.cups, self.goal_cups);
        for h in &self.habits {
            let d = if h.done { "1" } else { "0" };
            out.push_str(&format!("{d}|{}|{}\n", h.streak, h.title));
        }
        let _ = std::fs::write(p, out);
    }

    fn add_cup(&mut self, delta: i32) {
        let next = (self.cups as i32 + delta).clamp(0, 20) as u32;
        self.cups = next;
        if let Some(first) = self.habits.first_mut() {
            first.done = self.cups >= self.goal_cups;
        }
        self.save();
    }
}

impl Module for HabitWater {
    fn id(&self) -> &'static str {
        "habit_water"
    }

    fn name(&self) -> &str {
        "Hydration & Habits"
    }

    fn icon(&self) -> &'static str {
        icons::DROP
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(620.0, 380.0))
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        let rect = ui.max_rect();
        let theme = ctx.theme;
        let size = kit::tile_size(rect);
        let resp = ui.interact(
            rect.expand(10.0),
            ui.id().with("habit-water-tile"),
            Sense::click(),
        );

        let ratio = (self.cups as f32 / self.goal_cups.max(1) as f32).clamp(0.0, 1.0);
        let water_col = Color32::from_rgb(56, 189, 248);

        match size {
            TileSize::Small => {
                let c = rect.center();
                let r = rect.width().min(rect.height()) * 0.5 - 2.0;
                paint::ring(ui.painter(), c, r, 3.0, ratio, kit::wash(theme), water_col);
                kit::text(
                    ui.painter(),
                    c,
                    Align2::CENTER_CENTER,
                    &format!("{}/{}", self.cups, self.goal_cups),
                    Theme::font_semibold(10.5),
                    theme.text,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::DROP, self.name(), self.cups >= self.goal_cups, theme);
                let plus_r = Rect::from_min_size(
                    pos2(rect.max.x - 28.0, rect.min.y + 19.0),
                    vec2(28.0, 20.0),
                );
                let plus_resp = ui.interact(plus_r, ui.id().with("hw-plus"), Sense::click());
                let p = ui.painter();
                p.rect_filled(
                    plus_r,
                    6.0,
                    with_alpha(water_col, if plus_resp.hovered() { 0.35 } else { 0.20 }),
                );
                kit::text(
                    p,
                    plus_r.center(),
                    Align2::CENTER_CENTER,
                    "+1",
                    Theme::font_semibold(11.0),
                    water_col,
                );
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 29.0),
                    Align2::LEFT_CENTER,
                    &format!(
                        "{}/{} {} ({}ml)",
                        self.cups,
                        self.goal_cups,
                        nook_ui::i18n::tr("Cups"),
                        self.cups * 250
                    ),
                    Theme::font_semibold(SIZE_LABEL),
                    theme.text,
                );
                if plus_resp.clicked() {
                    self.add_cup(1);
                    return;
                }
            }
            TileSize::Large => {
                tile_header(ui, icons::DROP, self.name(), self.cups >= self.goal_cups, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 30.0),
                    Align2::LEFT_CENTER,
                    &format!(
                        "{}/{} {} · {} ml",
                        self.cups,
                        self.goal_cups,
                        nook_ui::i18n::tr("Cups"),
                        self.cups * 250
                    ),
                    Theme::font_semibold(SIZE_BODY),
                    water_col,
                );
                let bar_r = Rect::from_min_size(
                    pos2(rect.min.x, rect.min.y + 44.0),
                    vec2(rect.width(), 6.0),
                );
                kit::bar(p, bar_r, ratio, water_col, theme);
                let done_cnt = self.habits.iter().filter(|h| h.done).count();
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 64.0),
                    Align2::LEFT_CENTER,
                    &format!(
                        "{}: {}/{}",
                        nook_ui::i18n::tr("Daily Habits"),
                        done_cnt,
                        self.habits.len()
                    ),
                    Theme::font(SIZE_LABEL),
                    theme.text_dim,
                );
                let btn_r = Rect::from_min_size(
                    pos2(rect.min.x, rect.max.y - 24.0),
                    vec2(rect.width(), 22.0),
                );
                if kit::button(
                    ui,
                    btn_r,
                    &nook_ui::i18n::tr("+1 Cup (250ml)"),
                    Some(icons::DROP),
                    true,
                    theme,
                )
                .clicked()
                {
                    self.add_cup(1);
                    return;
                }
            }
        }

        if resp.clicked() {
            self.open_page = true;
        }
    }

    fn ui_page(&mut self, ui: &mut Ui, header: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let mut cup_delta = 0i32;
        if kit::pill(header, "-1", false, theme).clicked() {
            cup_delta = -1;
        }
        header.add_space(6.0);
        if kit::pill(header, &nook_ui::i18n::tr("+1 Cup (250ml)"), true, theme).clicked() {
            cup_delta = 1;
        }
        if cup_delta != 0 {
            self.add_cup(cup_delta);
        }

        let water_col = Color32::from_rgb(56, 189, 248);
        let ratio = (self.cups as f32 / self.goal_cups.max(1) as f32).clamp(0.0, 1.0);

        // Top Hydration Card
        let hyd_card = Rect::from_min_size(body.min, vec2(body.width(), 94.0));
        kit::card(ui.painter(), hyd_card, theme);

        let ring_c = pos2(hyd_card.left() + 46.0, hyd_card.center().y);
        paint::ring(
            ui.painter(),
            ring_c,
            30.0,
            5.0,
            ratio,
            kit::wash(theme),
            water_col,
        );
        kit::text(
            ui.painter(),
            ring_c,
            Align2::CENTER_CENTER,
            &format!("{}%", (ratio * 100.0).round() as u32),
            Theme::font_semibold(12.0),
            water_col,
        );

        kit::text(
            ui.painter(),
            pos2(hyd_card.left() + 92.0, hyd_card.top() + 24.0),
            Align2::LEFT_CENTER,
            &format!(
                "{} / {} {} ({} / {} ml)",
                self.cups,
                self.goal_cups,
                nook_ui::i18n::tr("Cups"),
                self.cups * 250,
                self.goal_cups * 250
            ),
            Theme::font_semibold(SIZE_BODY + 3.0),
            theme.text,
        );
        let bar_r = Rect::from_min_size(
            pos2(hyd_card.left() + 92.0, hyd_card.top() + 48.0),
            vec2(hyd_card.width() - 230.0, 8.0),
        );
        kit::bar(ui.painter(), bar_r, ratio, water_col, theme);

        let add_btn = Rect::from_min_size(
            pos2(hyd_card.right() - 124.0, hyd_card.center().y - 14.0),
            vec2(110.0, 28.0),
        );
        if kit::button(
            ui,
            add_btn,
            &nook_ui::i18n::tr("+1 Cup (250ml)"),
            Some(icons::DROP),
            true,
            theme,
        )
        .clicked()
        {
            self.add_cup(1);
        }

        // Bottom Daily Habits Card
        let hab_top = hyd_card.bottom() + 10.0;
        let hab_card = Rect::from_min_max(pos2(body.left(), hab_top), body.max);
        kit::card(ui.painter(), hab_card, theme);

        kit::text(
            ui.painter(),
            pos2(hab_card.left() + 14.0, hab_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &nook_ui::i18n::tr("Daily Habits"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        // Add habit input row
        let input_r = Rect::from_min_size(
            pos2(hab_card.left() + 12.0, hab_card.top() + 30.0),
            vec2(hab_card.width() - 104.0, 28.0),
        );
        let hint = nook_ui::i18n::tr("New daily habit...");
        let resp_in = kit::field(
            ui,
            input_r,
            &mut self.new_habit,
            &hint,
            Id::new("habit-new-input"),
            Theme::font(SIZE_LABEL),
            theme,
        );
        let add_hab_btn = Rect::from_min_size(
            pos2(input_r.right() + 8.0, hab_card.top() + 30.0),
            vec2(72.0, 28.0),
        );
        let submit = (resp_in.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || kit::button(
                ui,
                add_hab_btn,
                &nook_ui::i18n::tr("Add"),
                Some(icons::PLUS),
                true,
                theme,
            )
            .clicked();
        if submit && !self.new_habit.trim().is_empty() {
            self.habits.push(Habit {
                title: self.new_habit.trim().to_owned(),
                done: false,
                streak: 1,
            });
            self.new_habit.clear();
            self.save();
        }

        let mut toggle_idx = None;
        let mut del_idx = None;
        let mut ry = hab_card.top() + 66.0;
        for (i, h) in self.habits.iter().enumerate() {
            if ry + 28.0 > hab_card.bottom() - 6.0 {
                break;
            }
            let row_r = Rect::from_min_size(
                pos2(hab_card.left() + 12.0, ry),
                vec2(hab_card.width() - 24.0, 26.0),
            );
            let del_r = Rect::from_center_size(
                pos2(row_r.right() - 14.0, row_r.center().y),
                vec2(22.0, 22.0),
            );
            let main_r = Rect::from_min_max(row_r.min, pos2(del_r.left() - 4.0, row_r.max.y));
            let resp = kit::row(ui, main_r, Id::new(("hab-row", i)), h.done, theme);
            let del_resp = ui.interact(del_r, Id::new(("hab-del", i)), Sense::click());

            let p = ui.painter();
            let chk = pos2(row_r.left() + 14.0, row_r.center().y);
            if h.done {
                p.circle_filled(chk, 7.0, Color32::from_rgb(52, 211, 153));
                paint::icon(p, chk, icons::CHECK, 10.0, Color32::WHITE, true);
            } else {
                p.circle_stroke(chk, 7.0, egui::Stroke::new(1.3, theme.text_dim));
            }
            kit::text(
                p,
                pos2(row_r.left() + 30.0, row_r.center().y),
                Align2::LEFT_CENTER,
                &h.title,
                Theme::font_semibold(SIZE_LABEL + 0.5),
                if h.done { theme.text_dim } else { theme.text },
            );
            kit::text(
                p,
                pos2(del_r.left() - 12.0, row_r.center().y),
                Align2::RIGHT_CENTER,
                &format!("🔥 {}d", h.streak),
                Theme::font_semibold(SIZE_LABEL),
                Color32::from_rgb(251, 146, 60),
            );
            paint::icon(
                p,
                del_r.center(),
                icons::TRASH,
                12.0,
                if del_resp.hovered() {
                    Color32::from_rgb(248, 113, 113)
                } else {
                    theme.text_faint
                },
                false,
            );
            if resp.clicked() {
                toggle_idx = Some(i);
            }
            if del_resp.clicked() {
                del_idx = Some(i);
            }
            ry += 30.0;
        }

        if let Some(i) = toggle_idx {
            self.habits[i].done = !self.habits[i].done;
            self.save();
        }
        if let Some(i) = del_idx {
            self.habits.remove(i);
            self.save();
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_cups_clamp_properly() {
        let mut hw = HabitWater::default();
        hw.cups = 0;
        hw.add_cup(-2);
        assert_eq!(hw.cups, 0);
        hw.add_cup(3);
        assert_eq!(hw.cups, 3);
    }
}
