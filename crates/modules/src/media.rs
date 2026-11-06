//! Media: now playing from any app that reports to Windows (Spotify,
//! browsers, the Media Player...) through SMTC, with album art, progress and
//! previous / play-pause / next. A note glyph shows in the pill while
//! something plays.
//!
//! The SMTC work runs on `nook_platform::media`'s worker thread; this module
//! only drains its snapshots in `on_tick` and sends commands back.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Align2, Color32, ColorImage, Pos2, Rect, Response, Sense, TextureHandle, TextureOptions, Ui,
    UiBuilder, pos2, vec2,
};
use nook_platform::media::{MediaCommand, MediaWatcher, NowPlaying};
use nook_ui::module::{Badge, Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::{icons, paint, widgets};
use std::sync::Arc;
use std::time::Duration;

const ART: f32 = 44.0;
const PAGE_ART: f32 = 136.0;

#[derive(Default)]
pub struct Media {
    watcher: Option<MediaWatcher>,
    now: Option<NowPlaying>,
    /// Texture for `now.art`, keyed by the art's allocation.
    art: Option<(usize, TextureHandle)>,
    ctx: Option<egui::Context>,
    open_page: bool,
}

impl Media {
    fn send(&self, cmd: MediaCommand) {
        if let Some(w) = &self.watcher {
            w.send(cmd);
        }
    }

    fn art_texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureId> {
        let art = self.now.as_ref()?.art.as_ref()?;
        let key = Arc::as_ptr(art) as usize;
        if self.art.as_ref().map(|(k, _)| *k) != Some(key) {
            let image =
                ColorImage::from_rgba_unmultiplied([art.width, art.height], art.rgba.as_slice());
            let tex = ctx.load_texture("nook-media-art", image, TextureOptions::LINEAR);
            self.art = Some((key, tex));
        }
        self.art.as_ref().map(|(_, t)| t.id())
    }
}

/// Single-line text, elided with "…" to `width` (with Arabic shaping).
fn text_line(ui: &Ui, pos: Pos2, text: &str, width: f32, font: egui::FontId, color: Color32) {
    let shaped = nook_ui::arabic::shape_bidi(text);
    let mut job = LayoutJob::single_section(shaped, TextFormat::simple(font, color));
    job.wrap = TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(pos, galley, color);
}

fn button_at(
    ui: &mut Ui,
    center: Pos2,
    size: f32,
    glyph: &'static str,
    active: bool,
    theme: &Theme,
) -> Response {
    let rect = Rect::from_center_size(center, vec2(size, size));
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(glyph));
    widgets::icon_button(&mut child, glyph, size, active, theme)
}

fn mmss(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Rounded image: a squircle-ish clip is overkill at 44 px, a rounded rect
/// mesh reads the same.
fn rounded_image(ui: &Ui, rect: Rect, tex: egui::TextureId, radius: f32) {
    ui.painter().add(
        egui::epaint::RectShape::filled(rect, radius, Color32::WHITE)
            .with_texture(tex, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
    );
}

impl Module for Media {
    fn id(&self) -> &'static str {
        "media"
    }

    fn name(&self) -> &str {
        "Media"
    }

    fn icon(&self) -> &'static str {
        icons::MUSIC_NOTES
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Large
    }

    fn is_active(&self) -> bool {
        self.now.as_ref().is_some_and(|n| n.playing)
    }

    fn collapsed_badge(&self) -> Option<Badge> {
        let n = self.now.as_ref()?;
        if !n.playing {
            return None;
        }
        let title = if !n.title.trim().is_empty() {
            n.title.clone()
        } else if !n.source.trim().is_empty() {
            n.source.clone()
        } else {
            "Now playing".to_owned()
        };
        Some(Badge::Media {
            title,
            artist: n.artist.clone(),
            playing: n.playing,
            art: self.art.as_ref().map(|(_, t)| t.id()),
        })
    }

    fn attach(&mut self, ctx: &egui::Context) {
        self.ctx = Some(ctx.clone());
        let ctx = ctx.clone();
        match MediaWatcher::spawn(move || ctx.request_repaint()) {
            Ok(w) => self.watcher = Some(w),
            Err(e) => tracing::warn!("media: {e}"),
        }
    }

    fn on_tick(&mut self) {
        if let Some(snapshot) = self.watcher.as_ref().and_then(MediaWatcher::poll) {
            self.now = snapshot;
            if self.now.as_ref().and_then(|n| n.art.as_ref()).is_none() {
                self.art = None;
            }
        }
        if let Some(ctx) = self.ctx.clone() {
            let _ = self.art_texture(&ctx);
        }
    }

    fn next_repaint(&self, tile_visible: bool) -> Option<Duration> {
        // The progress bar and time advance once a second while visible.
        let n = self.now.as_ref()?;
        (tile_visible && n.playing && n.timeline.is_some()).then(|| Duration::from_secs(1))
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 236.0))
    }

    fn ui_page(
        &mut self,
        ui: &mut Ui,
        header: &mut Ui,
        body: Rect,
        theme: &Theme,
    ) -> bool {
        let Some(now) = self.now.clone() else {
            let c = body.center();
            paint::icon(
                ui.painter(),
                c - vec2(0.0, 12.0),
                icons::MUSIC_NOTES,
                32.0,
                theme.text_faint,
                false,
            );
            ui.painter().text(
                c + vec2(0.0, 18.0),
                Align2::CENTER_CENTER,
                "Nothing playing right now",
                Theme::font(SIZE_BODY),
                theme.text_dim,
            );
            return false;
        };

        if !now.source.is_empty() {
            header.label(
                egui::RichText::new(&now.source)
                    .font(Theme::font_semibold(SIZE_LABEL))
                    .color(theme.accent_fg()),
            );
        }

        let inner = body.shrink2(vec2(6.0, 4.0));
        let art_rect = Rect::from_min_size(
            pos2(inner.min.x, inner.center().y - PAGE_ART * 0.5),
            vec2(PAGE_ART, PAGE_ART),
        );
        match self.art_texture(ui.ctx()) {
            Some(tex) => {
                rounded_image(ui, art_rect, tex, 16.0);
                ui.painter().rect_stroke(
                    art_rect,
                    16.0,
                    egui::Stroke::new(1.0, with_alpha(theme.tile_rim, 0.7)),
                    egui::StrokeKind::Inside,
                );
            }
            None => {
                widgets::card(ui.painter(), art_rect, theme);
                paint::icon(
                    ui.painter(),
                    art_rect.center(),
                    icons::MUSIC_NOTES,
                    40.0,
                    theme.text_faint,
                    false,
                );
            }
        }

        let col_x = art_rect.max.x + 18.0;
        let col_w = (inner.max.x - col_x).max(80.0);
        let top_y = art_rect.min.y + 4.0;

        let title = if now.title.is_empty() {
            "Unknown title"
        } else {
            &now.title
        };
        text_line(
            ui,
            pos2(col_x, top_y),
            title,
            col_w,
            Theme::font_semibold(SIZE_BODY + 3.0),
            theme.text,
        );
        let artist = if now.artist.is_empty() {
            &now.source
        } else {
            &now.artist
        };
        text_line(
            ui,
            pos2(col_x, top_y + 24.0),
            artist,
            col_w,
            Theme::font(SIZE_BODY),
            theme.text_dim,
        );

        // Interactive seek bar & timestamps.
        let bar_y = top_y + 62.0;
        if let Some((pos, dur)) = now.position() {
            let track_rect = Rect::from_min_size(pos2(col_x, bar_y), vec2(col_w, 6.0));
            let hit_rect = track_rect.expand2(vec2(4.0, 8.0));
            let resp = ui.interact(
                hit_rect,
                egui::Id::new("nook-media-page-seek"),
                Sense::click_and_drag(),
            );
            let mut frac = (pos / dur.max(1.0)).clamp(0.0, 1.0) as f32;
            if (resp.dragged() || resp.clicked())
                && let Some(p) = resp.interact_pointer_pos()
            {
                frac = ((p.x - track_rect.min.x) / track_rect.width().max(1.0)).clamp(0.0, 1.0);
                let target = f64::from(frac) * dur;
                self.send(MediaCommand::Seek(target));
                if let Some(n) = &mut self.now {
                    n.timeline = Some((target, dur));
                    n.captured = std::time::Instant::now();
                }
            }

            ui.painter()
                .rect_filled(track_rect, 3.0, with_alpha(theme.text_faint, 0.28));
            let mut fill = track_rect;
            fill.set_width(track_rect.width() * frac);
            ui.painter().rect_filled(fill, 3.0, theme.accent_fg());
            if resp.hovered() || resp.dragged() {
                let knob_x = track_rect.min.x + track_rect.width() * frac;
                ui.painter()
                    .circle_filled(pos2(knob_x, track_rect.center().y), 5.5, theme.text);
            }

            ui.painter().text(
                pos2(col_x, bar_y + 16.0),
                Align2::LEFT_CENTER,
                mmss(f64::from(frac) * dur),
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
            ui.painter().text(
                pos2(col_x + col_w, bar_y + 16.0),
                Align2::RIGHT_CENTER,
                mmss(dur),
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
        } else {
            ui.painter().text(
                pos2(col_x, bar_y + 6.0),
                Align2::LEFT_CENTER,
                if now.playing { "Playing live" } else { "Paused" },
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
        }

        // Transport controls centered in the right column bottom.
        let ctrl_y = art_rect.max.y - 16.0;
        let cx = col_x + col_w * 0.5;
        let gap = 44.0;
        if now.can_previous
            && button_at(ui, pos2(cx - gap, ctrl_y), 30.0, icons::SKIP_BACK, false, theme)
                .on_hover_text("Previous")
                .clicked()
        {
            self.send(MediaCommand::Previous);
        }
        let (glyph, tip) = if now.playing {
            (icons::PAUSE, "Pause")
        } else {
            (icons::PLAY, "Play")
        };
        if now.can_toggle
            && button_at(ui, pos2(cx, ctrl_y), 36.0, glyph, true, theme)
                .on_hover_text(tip)
                .clicked()
        {
            self.send(MediaCommand::PlayPause);
            if let Some(n) = &mut self.now {
                if let Some((pos, dur)) = n.position() {
                    n.timeline = Some((pos, dur));
                    n.captured = std::time::Instant::now();
                }
                n.playing = !n.playing;
            }
        }
        if now.can_next
            && button_at(ui, pos2(cx + gap, ctrl_y), 30.0, icons::SKIP_FORWARD, false, theme)
                .on_hover_text("Next")
                .clicked()
        {
            self.send(MediaCommand::Next);
        }

        false
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        let theme = ctx.theme;
        let rect = ui.max_rect();

        let Some(now) = self.now.clone() else {
            let c = rect.center();
            paint::icon(
                ui.painter(),
                c - vec2(0.0, 8.0),
                icons::MUSIC_NOTES,
                24.0,
                theme.text_faint,
                false,
            );
            ui.painter().text(
                c + vec2(0.0, 18.0),
                Align2::CENTER_CENTER,
                "Nothing playing",
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
            return;
        };

        // Album art, or a placeholder.
        let art_rect = Rect::from_min_size(rect.min, vec2(ART, ART));
        match self.art_texture(ui.ctx()) {
            Some(tex) => rounded_image(ui, art_rect, tex, 10.0),
            None => {
                widgets::card(ui.painter(), art_rect, theme);
                paint::icon(
                    ui.painter(),
                    art_rect.center(),
                    icons::MUSIC_NOTES,
                    20.0,
                    theme.text_faint,
                    false,
                );
            }
        }

        // Right of the art: source app, time, progress bar.
        let col = rect.min.x + ART + 8.0;
        let col_w = rect.max.x - col;
        text_line(
            ui,
            pos2(col, rect.min.y + 2.0),
            &now.source,
            col_w,
            Theme::font(SIZE_LABEL - 1.0),
            theme.text_faint,
        );
        if let Some((pos, dur)) = now.position() {
            ui.painter().text(
                pos2(col, rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                format!("{} / {}", mmss(pos), mmss(dur)),
                Theme::font(SIZE_LABEL - 1.0),
                theme.text_dim,
            );
            let bar = Rect::from_min_size(pos2(col, rect.min.y + 36.0), vec2(col_w, 3.0));
            ui.painter()
                .rect_filled(bar, 1.5, with_alpha(theme.text_faint, 0.3));
            let frac = (pos / dur.max(1.0)).clamp(0.0, 1.0) as f32;
            let mut fill = bar;
            fill.set_width(bar.width() * frac);
            ui.painter().rect_filled(fill, 1.5, theme.accent_fg());
        } else {
            ui.painter().text(
                pos2(col, rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                if now.playing { "Playing" } else { "Paused" },
                Theme::font(SIZE_LABEL - 1.0),
                theme.text_dim,
            );
        }

        // Title and artist.
        let title = if now.title.is_empty() {
            "Unknown title"
        } else {
            &now.title
        };
        text_line(
            ui,
            pos2(rect.min.x, rect.min.y + ART + 6.0),
            title,
            rect.width(),
            Theme::font_semibold(SIZE_LABEL + 1.0),
            theme.text,
        );
        text_line(
            ui,
            pos2(rect.min.x, rect.min.y + ART + 21.0),
            &now.artist,
            rect.width(),
            Theme::font(SIZE_LABEL),
            theme.text_dim,
        );

        // Transport.
        let mut transport_clicked = false;
        let y = rect.max.y - 11.0;
        let cx = rect.center().x;
        let gap = 34.0;
        if now.can_previous
            && button_at(ui, pos2(cx - gap, y), 24.0, icons::SKIP_BACK, false, theme)
                .on_hover_text("Previous")
                .clicked()
        {
            self.send(MediaCommand::Previous);
            transport_clicked = true;
        }
        let (glyph, tip) = if now.playing {
            (icons::PAUSE, "Pause")
        } else {
            (icons::PLAY, "Play")
        };
        if now.can_toggle
            && button_at(ui, pos2(cx, y), 28.0, glyph, true, theme)
                .on_hover_text(tip)
                .clicked()
        {
            self.send(MediaCommand::PlayPause);
            transport_clicked = true;
            // Flip right away; the next snapshot confirms it.
            if let Some(n) = &mut self.now {
                if let Some((pos, dur)) = n.position() {
                    n.timeline = Some((pos, dur));
                    n.captured = std::time::Instant::now();
                }
                n.playing = !n.playing;
            }
        }
        if now.can_next
            && button_at(
                ui,
                pos2(cx + gap, y),
                24.0,
                icons::SKIP_FORWARD,
                false,
                theme,
            )
            .on_hover_text("Next")
            .clicked()
        {
            self.send(MediaCommand::Next);
            transport_clicked = true;
        }

        // Clicking anywhere on the upper part of the media tile opens the full Media Player page.
        let open_hit = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - 26.0));
        if !transport_clicked
            && ui
                .interact(open_hit, egui::Id::new("nook-media-open-page"), Sense::click())
                .on_hover_text("Open Media Player")
                .clicked()
        {
            self.open_page = true;
        }
    }
}
