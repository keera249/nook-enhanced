//! Smart Unit Converter: instant multi-unit conversion across Length, Weight,
//! Temperature, Data Storage, and Speed with one-click copy (inspired by
//! NotchPixel's Calculator & Unit Conversion engine).

use crate::kit;
use egui::{Align2, Rect, Sense, TextEdit, Ui, UiBuilder, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Length,
    Weight,
    Temperature,
    Data,
    Speed,
}

const CATEGORIES: &[Category] = &[
    Category::Length,
    Category::Weight,
    Category::Temperature,
    Category::Data,
    Category::Speed,
];

impl Category {
    pub fn name(self) -> &'static str {
        match self {
            Category::Length => "Length",
            Category::Weight => "Weight",
            Category::Temperature => "Temp",
            Category::Data => "Data",
            Category::Speed => "Speed",
        }
    }

    pub fn units(self) -> &'static [(&'static str, f64)] {
        match self {
            // Factor = base units (meters) per 1 unit
            Category::Length => &[
                ("m", 1.0),
                ("km", 1000.0),
                ("cm", 0.01),
                ("ft", 0.3048),
                ("in", 0.0254),
                ("mi", 1609.344),
            ],
            // Factor = grams per 1 unit
            Category::Weight => &[
                ("kg", 1000.0),
                ("g", 1.0),
                ("lb", 453.59237),
                ("oz", 28.349523),
            ],
            // Handled specially
            Category::Temperature => &[("°C", 1.0), ("°F", 1.0), ("K", 1.0)],
            // Factor = MB per 1 unit
            Category::Data => &[
                ("MB", 1.0),
                ("GB", 1024.0),
                ("TB", 1_048_576.0),
                ("KB", 1.0 / 1024.0),
            ],
            // Factor = m/s per 1 unit
            Category::Speed => &[
                ("km/h", 1.0 / 3.6),
                ("mph", 0.44704),
                ("m/s", 1.0),
                ("kn", 0.514444),
            ],
        }
    }
}

pub fn convert_value(cat: Category, val: f64, from_idx: usize, to_idx: usize) -> f64 {
    if cat == Category::Temperature {
        let celsius = match from_idx {
            0 => val,
            1 => (val - 32.0) * 5.0 / 9.0,
            _ => val - 273.15,
        };
        return match to_idx {
            0 => celsius,
            1 => celsius * 9.0 / 5.0 + 32.0,
            _ => celsius + 273.15,
        };
    }
    let units = cat.units();
    let f_from = units.get(from_idx).map(|u| u.1).unwrap_or(1.0);
    let f_to = units.get(to_idx).map(|u| u.1).unwrap_or(1.0).max(1e-12);
    (val * f_from) / f_to
}

pub struct UnitConverter {
    cat: Category,
    input: String,
    from_unit: usize,
    to_unit: usize,
    open_page: bool,
}

impl Default for UnitConverter {
    fn default() -> Self {
        Self {
            cat: Category::Length,
            input: "100".into(),
            from_unit: 1, // km
            to_unit: 5,   // mi
            open_page: false,
        }
    }
}

impl UnitConverter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Module for UnitConverter {
    fn id(&self) -> &'static str {
        "unit_converter"
    }

    fn name(&self) -> &str {
        "Unit Converter"
    }

    fn icon(&self) -> &'static str {
        icons::SCALES
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        let units = self.cat.units();
        let f_idx = self.from_unit.min(units.len() - 1);
        let t_idx = self.to_unit.min(units.len() - 1);
        let val = self.input.trim().parse::<f64>().unwrap_or(1.0);
        let out = convert_value(self.cat, val, f_idx, t_idx);
        let u_from = units[f_idx].0;
        let u_to = units[t_idx].0;
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::SCALES, "Units", false, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &format!("{out:.1} {u_to}"),
                    Theme::font_semibold(14.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!("{val:.0} {u_from}"),
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::SCALES, "Unit Converter", false, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &format!("{val} {u_from} = {out:.1} {u_to}"),
                    Theme::font_semibold(11.5),
                    theme.text,
                );
            }
            TileSize::Large => {
                tile_header(ui, icons::SCALES, "Unit Converter", false, theme);
                let p = ui.painter();
                let top = content.top() + 22.0;
                for (i, &(u_name, _)) in units.iter().take(4).enumerate() {
                    let v = convert_value(self.cat, val, f_idx, i);
                    kit::text(
                        p,
                        pos2(content.left(), top + i as f32 * 20.0),
                        Align2::LEFT_TOP,
                        &format!("{v:.2} {u_name}"),
                        Theme::font_semibold(11.5),
                        theme.text,
                    );
                }
            }
        }

        if ui
            .interact(rect, ui.id().with("uc-open"), Sense::click())
            .clicked()
        {
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 250.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, _actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        // Category tabs
        let cw = (body.width() - 24.0) / CATEGORIES.len() as f32;
        for (i, &c) in CATEGORIES.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(body.left() + i as f32 * (cw + 6.0), body.top()),
                vec2(cw, 24.0),
            );
            let resp = ui.interact(r, ui.id().with(("uc-cat", i)), Sense::click());
            let active = self.cat == c;
            let p = ui.painter();
            p.rect_filled(
                r,
                6.0,
                if active {
                    with_alpha(theme.accent, 0.36)
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
                &nook_ui::i18n::tr(c.name()),
                Theme::font_semibold(11.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() && self.cat != c {
                self.cat = c;
                self.from_unit = 0;
                self.to_unit = 1;
            }
        }

        let card = Rect::from_min_max(pos2(body.left(), body.top() + 32.0), body.max);
        kit::card(ui.painter(), card, theme);

        // Input value + Source unit selector
        let in_r = Rect::from_min_size(
            pos2(card.left() + 12.0, card.top() + 12.0),
            vec2(150.0, 28.0),
        );
        ui.painter().rect_filled(in_r, 6.0, theme.tile_bottom);
        let mut iui = ui.new_child(UiBuilder::new().max_rect(in_r.shrink2(vec2(8.0, 4.0))));
        iui.add(
            TextEdit::singleline(&mut self.input)
                .hint_text("100")
                .frame(egui::Frame::NONE)
                .font(Theme::font_semibold(SIZE_BODY)),
        );

        let units = self.cat.units();
        let mut ux = in_r.right() + 10.0;
        for (i, &(u_lbl, _)) in units.iter().enumerate() {
            let r = Rect::from_min_size(pos2(ux, card.top() + 12.0), vec2(48.0, 28.0));
            let resp = ui.interact(r, ui.id().with(("uc-fu", i)), Sense::click());
            let active = self.from_unit == i;
            let p = ui.painter();
            p.rect_filled(
                r,
                6.0,
                if active {
                    with_alpha(theme.accent, 0.35)
                } else {
                    theme.tile_bottom
                },
            );
            kit::text(
                p,
                r.center(),
                Align2::CENTER_CENTER,
                u_lbl,
                Theme::font_semibold(11.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() {
                self.from_unit = i;
            }
            ux += 54.0;
        }

        // Grid of converted results across all units in this category
        let val = self.input.trim().parse::<f64>().unwrap_or(0.0);
        let f_idx = self.from_unit.min(units.len() - 1);
        let grid_top = card.top() + 52.0;
        let col_w = (card.width() - 32.0) / 2.0;
        for (i, &(u_lbl, _)) in units.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let r = Rect::from_min_size(
                pos2(
                    card.left() + 12.0 + col as f32 * (col_w + 8.0),
                    grid_top + row as f32 * 36.0,
                ),
                vec2(col_w, 30.0),
            );
            let resp = ui.interact(r, ui.id().with(("uc-res", i)), Sense::click());
            let p = ui.painter();
            p.rect_filled(
                r,
                6.0,
                if resp.hovered() {
                    with_alpha(theme.accent, 0.25)
                } else {
                    theme.tile_bottom
                },
            );
            let converted = convert_value(self.cat, val, f_idx, i);
            let formatted = if converted.abs() >= 100.0 || converted == 0.0 {
                format!("{converted:.2}")
            } else {
                format!("{converted:.4}")
            };
            kit::text(
                p,
                pos2(r.left() + 10.0, r.center().y),
                Align2::LEFT_CENTER,
                u_lbl,
                Theme::font_semibold(SIZE_LABEL),
                theme.accent_fg(),
            );
            kit::text(
                p,
                pos2(r.right() - 10.0, r.center().y),
                Align2::RIGHT_CENTER,
                &formatted,
                Theme::font_semibold(SIZE_BODY),
                theme.text,
            );
            if resp.clicked() {
                self.to_unit = i;
                ui.ctx().copy_text(formatted.clone());
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_conversions_accurate() {
        let f = convert_value(Category::Temperature, 100.0, 0, 1);
        assert!((f - 212.0).abs() < 0.01);
        let m = convert_value(Category::Length, 1.0, 1, 0);
        assert!((m - 1000.0).abs() < 0.01);
    }
}
