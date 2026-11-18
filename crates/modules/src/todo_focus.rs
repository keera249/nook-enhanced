//! To-Do & Pomodoro Focus Studio: persistent task checklist (`%APPDATA%\Nook\todos.txt`)
//! paired with a multi-session Pomodoro Focus/Break timer that mirrors its
//! countdown ring into the collapsed Notch pill (inspired by NotchPixel's
//! TodoList and PomodoroTimer widgets).

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
pub struct TodoItem {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PomoPhase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl PomoPhase {
    pub fn label(self) -> &'static str {
        match self {
            PomoPhase::Focus => "Focus",
            PomoPhase::ShortBreak => "Break",
            PomoPhase::LongBreak => "Long Break",
        }
    }
}

pub struct TodoFocus {
    todos: Vec<TodoItem>,
    draft: String,
    phase: PomoPhase,
    focus_mins: u32,
    total_secs: u32,
    remaining_secs: f64,
    running_since: Option<Instant>,
    sessions_done: u32,
    open_page: bool,
}

impl Default for TodoFocus {
    fn default() -> Self {
        let todos = load_todos();
        Self {
            todos,
            draft: String::new(),
            phase: PomoPhase::Focus,
            focus_mins: 25,
            total_secs: 25 * 60,
            remaining_secs: 25.0 * 60.0,
            running_since: None,
            sessions_done: 0,
            open_page: false,
        }
    }
}

fn todos_path() -> Option<PathBuf> {
    nook_platform::files::config_dir().map(|d| d.join("todos.txt"))
}

fn load_todos() -> Vec<TodoItem> {
    let Some(path) = todos_path() else {
        return default_todos();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return default_todos();
    };
    let items: Vec<TodoItem> = raw
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            if let Some(rest) = t.strip_prefix("[x] ") {
                Some(TodoItem {
                    text: rest.trim().to_owned(),
                    done: true,
                })
            } else if let Some(rest) = t.strip_prefix("[ ] ") {
                Some(TodoItem {
                    text: rest.trim().to_owned(),
                    done: false,
                })
            } else if !t.is_empty() {
                Some(TodoItem {
                    text: t.to_owned(),
                    done: false,
                })
            } else {
                None
            }
        })
        .collect();
    if items.is_empty() {
        default_todos()
    } else {
        items
    }
}

fn default_todos() -> Vec<TodoItem> {
    vec![
        TodoItem {
            text: "Start a 25m Focus session".into(),
            done: false,
        },
        TodoItem {
            text: "Review daily priorities".into(),
            done: false,
        },
        TodoItem {
            text: "Organize workspace & notes".into(),
            done: true,
        },
    ]
}

fn save_todos(items: &[TodoItem]) {
    let Some(path) = todos_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = String::new();
    for item in items {
        let prefix = if item.done { "[x] " } else { "[ ] " };
        out.push_str(prefix);
        out.push_str(&item.text);
        out.push('\n');
    }
    let _ = std::fs::write(path, out);
}

impl TodoFocus {
    pub fn new() -> Self {
        Self::default()
    }

    fn tick_timer(&mut self) {
        if let Some(since) = self.running_since {
            let now = Instant::now();
            let dt = now.duration_since(since).as_secs_f64();
            self.running_since = Some(now);
            self.remaining_secs = (self.remaining_secs - dt).max(0.0);
            if self.remaining_secs <= 0.0 {
                self.running_since = None;
                system::alert_sound();
                match self.phase {
                    PomoPhase::Focus => {
                        self.sessions_done += 1;
                        if self.sessions_done % 4 == 0 {
                            self.set_phase(PomoPhase::LongBreak);
                        } else {
                            self.set_phase(PomoPhase::ShortBreak);
                        }
                    }
                    PomoPhase::ShortBreak | PomoPhase::LongBreak => {
                        self.set_phase(PomoPhase::Focus);
                    }
                }
            }
        }
    }

    fn set_phase(&mut self, phase: PomoPhase) {
        self.phase = phase;
        self.running_since = None;
        let mins = match phase {
            PomoPhase::Focus => self.focus_mins,
            PomoPhase::ShortBreak => 5,
            PomoPhase::LongBreak => 15,
        };
        self.total_secs = mins * 60;
        self.remaining_secs = f64::from(self.total_secs);
    }

    fn toggle_run(&mut self) {
        if self.running_since.is_some() {
            self.tick_timer();
            self.running_since = None;
        } else {
            if self.remaining_secs <= 0.5 {
                self.remaining_secs = f64::from(self.total_secs);
            }
            self.running_since = Some(Instant::now());
        }
    }

    fn format_remaining(&self) -> String {
        let secs = self.remaining_secs.ceil().max(0.0) as u32;
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    fn progress(&self) -> f32 {
        if self.total_secs == 0 {
            return 0.0;
        }
        (self.remaining_secs as f32 / self.total_secs as f32).clamp(0.0, 1.0)
    }
}

impl Module for TodoFocus {
    fn id(&self) -> &'static str {
        "todo_focus"
    }

    fn name(&self) -> &str {
        "Tasks & Focus"
    }

    fn icon(&self) -> &'static str {
        icons::CHECK_SQUARE_OFFSET
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn is_active(&self) -> bool {
        self.running_since.is_some()
    }

    fn collapsed_badge(&self) -> Option<Badge> {
        if self.running_since.is_some() {
            Some(Badge::Ring {
                label: self.format_remaining(),
                progress: self.progress(),
            })
        } else {
            None
        }
    }

    fn next_repaint(&self, _tile_visible: bool) -> Option<Duration> {
        self.running_since.map(|_| Duration::from_millis(250))
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        self.tick_timer();
        let running = self.running_since.is_some();
        let pending = self.todos.iter().filter(|t| !t.done).count();
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::CHECK_SQUARE_OFFSET, "Focus", running, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &self.format_remaining(),
                    Theme::font_semibold(15.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!("{pending} {}", nook_ui::i18n::tr("tasks")),
                    Theme::font(10.0),
                    theme.text_dim,
                );
                if ui
                    .interact(rect, ui.id().with("tf-sm"), Sense::click())
                    .clicked()
                {
                    self.open_page = true;
                }
            }
            TileSize::Wide => {
                tile_header(ui, icons::CHECK_SQUARE_OFFSET, "Tasks & Focus", running, theme);
                let y = content.top() + 29.0;
                let play_r =
                    Rect::from_min_size(pos2(content.right() - 20.0, y - 10.0), vec2(20.0, 20.0));
                let play_resp = ui.interact(play_r, ui.id().with("tf-w-play"), Sense::click());
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &self.format_remaining(),
                    Theme::font_semibold(13.5),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(play_r.left() - 6.0, y),
                    Align2::RIGHT_CENTER,
                    &format!("{pending} {}", nook_ui::i18n::tr("tasks")),
                    Theme::font(10.0),
                    theme.text_dim,
                );
                p.rect_filled(play_r, 10.0, with_alpha(theme.accent, 0.32));
                kit::text(
                    p,
                    play_r.center(),
                    Align2::CENTER_CENTER,
                    if running { icons::PAUSE } else { icons::PLAY },
                    Theme::font(10.5),
                    theme.accent_fg(),
                );
                if play_resp.clicked() {
                    self.toggle_run();
                } else if ui
                    .interact(
                        Rect::from_min_size(rect.min, vec2(rect.width() - 32.0, rect.height())),
                        ui.id().with("tf-w-open"),
                        Sense::click(),
                    )
                    .clicked()
                {
                    self.open_page = true;
                }
            }
            TileSize::Large => {
                tile_header(ui, icons::CHECK_SQUARE_OFFSET, "Tasks & Focus", running, theme);
                let top = content.top() + 21.0;

                // Pomodoro bar row
                let pomo_r = Rect::from_min_size(
                    pos2(content.left(), top),
                    vec2(content.width(), 26.0),
                );
                let btn_r = Rect::from_min_size(
                    pos2(pomo_r.right() - 24.0, pomo_r.top() + 2.0),
                    vec2(22.0, 22.0),
                );
                let btn_resp = ui.interact(btn_r, ui.id().with("tf-lg-play"), Sense::click());
                let p = ui.painter();
                p.rect_filled(pomo_r, 7.0, theme.tile_bottom);
                kit::text(
                    p,
                    pos2(pomo_r.left() + 7.0, pomo_r.center().y),
                    Align2::LEFT_CENTER,
                    &self.format_remaining(),
                    Theme::font_semibold(13.5),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(pomo_r.left() + 50.0, pomo_r.center().y),
                    Align2::LEFT_CENTER,
                    &nook_ui::i18n::tr(self.phase.label()),
                    Theme::font(10.0),
                    theme.accent_fg(),
                );
                p.rect_filled(btn_r, 6.0, with_alpha(theme.accent, 0.35));
                kit::text(
                    p,
                    btn_r.center(),
                    Align2::CENTER_CENTER,
                    if running { icons::PAUSE } else { icons::PLAY },
                    Theme::font(11.0),
                    theme.accent_fg(),
                );

                // Top 3 checklist items
                let mut toggled = None;
                let list_top = top + 30.0;
                for (i, item) in self.todos.iter().take(3).enumerate() {
                    let ry = list_top + i as f32 * 19.0;
                    let row_r = Rect::from_min_size(
                        pos2(content.left(), ry),
                        vec2(content.width(), 18.0),
                    );
                    let resp = ui.interact(row_r, ui.id().with(("tf-chk", i)), Sense::click());
                    let p = ui.painter();
                    let ic = if item.done {
                        icons::CHECK_CIRCLE
                    } else {
                        icons::CIRCLE
                    };
                    let col = if item.done {
                        theme.text_faint
                    } else {
                        theme.text
                    };
                    let mut display_txt = nook_ui::i18n::tr(&item.text);
                    if display_txt.chars().count() > 19 {
                        display_txt = display_txt.chars().take(18).collect::<String>() + "…";
                    }
                    kit::text(
                        p,
                        pos2(row_r.left() + 2.0, row_r.center().y),
                        Align2::LEFT_CENTER,
                        &format!("{ic} {display_txt}"),
                        Theme::font(10.0),
                        col,
                    );
                    if resp.clicked() {
                        toggled = Some(i);
                    }
                }

                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() + 1.0),
                    Align2::LEFT_BOTTOM,
                    &format!(
                        "{pending} {} · {} {}",
                        nook_ui::i18n::tr("left"),
                        self.sessions_done,
                        nook_ui::i18n::tr("sessions")
                    ),
                    Theme::font(9.5),
                    theme.text_faint,
                );

                if btn_resp.clicked() {
                    self.toggle_run();
                } else if let Some(idx) = toggled {
                    if let Some(t) = self.todos.get_mut(idx) {
                        t.done = !t.done;
                        save_todos(&self.todos);
                    }
                } else {
                    let hdr = Rect::from_min_size(rect.min, vec2(rect.width(), 28.0));
                    let ftr = Rect::from_min_size(
                        pos2(rect.left(), rect.bottom() - 20.0),
                        vec2(rect.width(), 20.0),
                    );
                    if ui
                        .interact(hdr, ui.id().with("tf-lg-hdr"), Sense::click())
                        .clicked()
                        || ui
                            .interact(ftr, ui.id().with("tf-lg-ftr"), Sense::click())
                            .clicked()
                    {
                        self.open_page = true;
                    }
                }
            }
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 270.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        self.tick_timer();
        let running = self.running_since.is_some();

        if kit::pill(actions, "Clear Done", false, theme).clicked() {
            self.todos.retain(|t| !t.done);
            save_todos(&self.todos);
        }

        let p = ui.painter();
        let pomo_card = Rect::from_min_size(pos2(body.left(), body.top()), vec2(215.0, body.height()));
        let todo_card = Rect::from_min_max(pos2(pomo_card.right() + 10.0, body.top()), body.max);
        kit::card(p, pomo_card, theme);
        kit::card(p, todo_card, theme);

        // Left: Pomodoro Focus Studio
        kit::text(
            p,
            pos2(pomo_card.center().x, pomo_card.top() + 14.0),
            Align2::CENTER_TOP,
            &nook_ui::i18n::tr(self.phase.label()),
            Theme::font_semibold(SIZE_BODY),
            theme.accent_fg(),
        );
        kit::text(
            p,
            pos2(pomo_card.center().x, pomo_card.top() + 40.0),
            Align2::CENTER_TOP,
            &self.format_remaining(),
            Theme::font_semibold(36.0),
            theme.text,
        );

        let bar_r = Rect::from_min_size(
            pos2(pomo_card.left() + 16.0, pomo_card.top() + 88.0),
            vec2(pomo_card.width() - 32.0, 8.0),
        );
        kit::bar(p, bar_r, self.progress(), theme.accent_fg(), theme);

        // Phase selector pills
        let phases = [
            (PomoPhase::Focus, "Focus"),
            (PomoPhase::ShortBreak, "5m Break"),
            (PomoPhase::LongBreak, "15m Break"),
        ];
        let pw = (pomo_card.width() - 24.0 - 8.0) / 3.0;
        for (i, &(ph, lbl)) in phases.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(pomo_card.left() + 12.0 + i as f32 * (pw + 4.0), pomo_card.top() + 106.0),
                vec2(pw, 22.0),
            );
            let resp = ui.interact(r, ui.id().with(("pomo-ph", i)), Sense::click());
            let active = self.phase == ph;
            let p = ui.painter();
            p.rect_filled(
                r,
                6.0,
                if active {
                    with_alpha(theme.accent, 0.35)
                } else if resp.hovered() {
                    theme.tile_top
                } else {
                    theme.tile_bottom
                },
            );
            kit::text(
                p,
                r.center(),
                Align2::CENTER_CENTER,
                &nook_ui::i18n::tr(lbl),
                Theme::font_semibold(10.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() {
                self.set_phase(ph);
            }
        }

        // Duration presets for Focus
        for (i, &mins) in [15u32, 25, 45, 60].iter().enumerate() {
            let dw = (pomo_card.width() - 24.0 - 12.0) / 4.0;
            let r = Rect::from_min_size(
                pos2(pomo_card.left() + 12.0 + i as f32 * (dw + 4.0), pomo_card.top() + 134.0),
                vec2(dw, 20.0),
            );
            let resp = ui.interact(r, ui.id().with(("pomo-dur", mins)), Sense::click());
            let active = self.focus_mins == mins && self.phase == PomoPhase::Focus;
            let p = ui.painter();
            p.rect_filled(
                r,
                5.0,
                if active {
                    with_alpha(theme.accent, 0.25)
                } else {
                    theme.tile_bottom
                },
            );
            kit::text(
                p,
                r.center(),
                Align2::CENTER_CENTER,
                &format!("{mins}m"),
                Theme::font(10.0),
                if active { theme.accent_fg() } else { theme.text_faint },
            );
            if resp.clicked() {
                self.focus_mins = mins;
                self.set_phase(PomoPhase::Focus);
            }
        }

        // Start / Pause & Reset buttons
        let run_r = Rect::from_min_size(
            pos2(pomo_card.left() + 12.0, pomo_card.bottom() - 36.0),
            vec2(pomo_card.width() - 60.0, 26.0),
        );
        let rst_r = Rect::from_min_size(
            pos2(run_r.right() + 6.0, pomo_card.bottom() - 36.0),
            vec2(30.0, 26.0),
        );
        let run_resp = ui.interact(run_r, ui.id().with("pomo-run"), Sense::click());
        let rst_resp = ui.interact(rst_r, ui.id().with("pomo-rst"), Sense::click());
        let p = ui.painter();
        p.rect_filled(run_r, 7.0, with_alpha(theme.accent, 0.40));
        p.rect_filled(rst_r, 7.0, theme.tile_bottom);
        kit::text(
            p,
            run_r.center(),
            Align2::CENTER_CENTER,
            &nook_ui::i18n::tr(if running { "Pause" } else { "Start Focus" }),
            Theme::font_semibold(11.5),
            theme.text,
        );
        kit::text(
            p,
            rst_r.center(),
            Align2::CENTER_CENTER,
            icons::ARROW_COUNTER_CLOCKWISE,
            Theme::font(13.0),
            theme.text_dim,
        );
        if run_resp.clicked() {
            self.toggle_run();
        }
        if rst_resp.clicked() {
            self.set_phase(self.phase);
        }

        // Right: To-Do List + Add input
        let input_r = Rect::from_min_size(
            pos2(todo_card.left() + 10.0, todo_card.top() + 10.0),
            vec2(todo_card.width() - 52.0, 26.0),
        );
        let add_r = Rect::from_min_size(
            pos2(input_r.right() + 6.0, todo_card.top() + 10.0),
            vec2(26.0, 26.0),
        );
        ui.painter().rect_filled(input_r, 6.0, theme.tile_bottom);
        let mut iui = ui.new_child(UiBuilder::new().max_rect(input_r.shrink2(vec2(6.0, 3.0))));
        let edit = iui.add(
            TextEdit::singleline(&mut self.draft)
                .hint_text(nook_ui::i18n::tr("Add a new task…"))
                .frame(egui::Frame::NONE)
                .font(Theme::font(SIZE_BODY)),
        );
        let enter_pressed = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let add_resp = ui.interact(add_r, ui.id().with("todo-add"), Sense::click());
        ui.painter()
            .rect_filled(add_r, 6.0, with_alpha(theme.accent, 0.35));
        kit::text(
            ui.painter(),
            add_r.center(),
            Align2::CENTER_CENTER,
            icons::PLUS,
            Theme::font_semibold(14.0),
            theme.accent_fg(),
        );
        if (enter_pressed || add_resp.clicked()) && !self.draft.trim().is_empty() {
            self.todos.insert(
                0,
                TodoItem {
                    text: self.draft.trim().to_owned(),
                    done: false,
                },
            );
            self.draft.clear();
            save_todos(&self.todos);
        }

        let list_r = Rect::from_min_max(
            pos2(todo_card.left() + 10.0, todo_card.top() + 44.0),
            pos2(todo_card.right() - 10.0, todo_card.bottom() - 8.0),
        );
        let mut lui = ui.new_child(UiBuilder::new().max_rect(list_r));
        let mut toggle_idx = None;
        let mut remove_idx = None;
        ScrollArea::vertical()
            .id_salt("todo-scroll")
            .max_height(list_r.height())
            .show(&mut lui, |ui| {
                for (idx, item) in self.todos.iter().enumerate() {
                    let (row, _) =
                        ui.allocate_exact_size(vec2(list_r.width() - 6.0, 26.0), Sense::hover());
                    let chk_r = Rect::from_min_size(row.min, vec2(row.width() - 26.0, row.height()));
                    let del_r = Rect::from_min_size(
                        pos2(row.right() - 22.0, row.top() + 3.0),
                        vec2(20.0, 20.0),
                    );
                    let chk_resp = ui.interact(chk_r, ui.id().with(("td-c", idx)), Sense::click());
                    let del_resp = ui.interact(del_r, ui.id().with(("td-d", idx)), Sense::click());
                    let p = ui.painter();
                    if chk_resp.hovered() {
                        p.rect_filled(row, 6.0, theme.tile_bottom);
                    }
                    let ic = if item.done {
                        icons::CHECK_CIRCLE
                    } else {
                        icons::CIRCLE
                    };
                    kit::text(
                        p,
                        pos2(row.left() + 6.0, row.center().y),
                        Align2::LEFT_CENTER,
                        &format!("{ic}  {}", nook_ui::i18n::tr(&item.text)),
                        Theme::font(SIZE_BODY),
                        if item.done { theme.text_faint } else { theme.text },
                    );
                    kit::text(
                        p,
                        del_r.center(),
                        Align2::CENTER_CENTER,
                        icons::TRASH,
                        Theme::font(12.0),
                        if del_resp.hovered() {
                            Color32::from_rgb(240, 100, 100)
                        } else {
                            theme.text_faint
                        },
                    );
                    if chk_resp.clicked() {
                        toggle_idx = Some(idx);
                    }
                    if del_resp.clicked() {
                        remove_idx = Some(idx);
                    }
                }
            });

        if let Some(i) = toggle_idx
            && let Some(item) = self.todos.get_mut(i)
        {
            item.done = !item.done;
            save_todos(&self.todos);
        }
        if let Some(i) = remove_idx
            && i < self.todos.len()
        {
            self.todos.remove(i);
            save_todos(&self.todos);
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pomo_phase_transition_resets_duration() {
        let mut tf = TodoFocus::default();
        tf.set_phase(PomoPhase::ShortBreak);
        assert_eq!(tf.total_secs, 5 * 60);
        assert_eq!(tf.format_remaining(), "05:00");
    }
}
