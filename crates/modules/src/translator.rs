//! Instant Translator: bidirectional multi-language translation with one-click
//! clipboard paste, language swap, and history (inspired by NotchPixel's
//! TranslatorWidget).

use crate::kit;
use egui::{Align2, Id, Rect, Sense, Ui, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::icons;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use std::sync::{Arc, Mutex};

const LANGUAGES: &[(&str, &str)] = &[
    ("ar", "العربية"),
    ("en", "English"),
    ("fr", "Français"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("tr", "Türkçe"),
    ("ru", "Русский"),
    ("ja", "日本語"),
];

#[derive(Clone, Debug)]
struct TranslationEntry {
    from_code: &'static str,
    to_code: &'static str,
    source: String,
    translated: String,
}

#[derive(Default)]
struct SharedTrans {
    busy: bool,
    result: String,
    error: Option<String>,
    history: Vec<TranslationEntry>,
}

pub struct Translator {
    from_idx: usize,
    to_idx: usize,
    input: String,
    state: Arc<Mutex<SharedTrans>>,
    ctx: Option<egui::Context>,
    open_page: bool,
}

impl Default for Translator {
    fn default() -> Self {
        let mut st = SharedTrans::default();
        st.result = "مرحباً بك في نووك المطور".into();
        st.history.push(TranslationEntry {
            from_code: "en",
            to_code: "ar",
            source: "Welcome to Nook Enhanced".into(),
            translated: "مرحباً بك في نووك المطور".into(),
        });
        Self {
            from_idx: 1, // English
            to_idx: 0,   // Arabic
            input: "Welcome to Nook Enhanced".into(),
            state: Arc::new(Mutex::new(st)),
            ctx: None,
            open_page: false,
        }
    }
}

impl Translator {
    fn trigger_translate(&mut self) {
        let text = self.input.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let from_code = LANGUAGES[self.from_idx].0;
        let to_code = LANGUAGES[self.to_idx].0;
        if let Ok(mut s) = self.state.lock() {
            if s.busy {
                return;
            }
            s.busy = true;
            s.error = None;
        }
        let st = Arc::clone(&self.state);
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = fetch_translation(&text, from_code, to_code);
            if let Ok(mut s) = st.lock() {
                s.busy = false;
                match res {
                    Ok(translated) => {
                        s.result = translated.clone();
                        s.history.retain(|h| h.source != text);
                        s.history.insert(
                            0,
                            TranslationEntry {
                                from_code,
                                to_code,
                                source: text,
                                translated,
                            },
                        );
                        if s.history.len() > 8 {
                            s.history.truncate(8);
                        }
                    }
                    Err(e) => {
                        s.error = Some(e);
                    }
                }
            }
            if let Some(c) = ctx {
                c.request_repaint();
            }
        });
    }

    fn swap_languages(&mut self) {
        std::mem::swap(&mut self.from_idx, &mut self.to_idx);
        if let Ok(s) = self.state.lock()
            && !s.result.trim().is_empty()
        {
            self.input = s.result.clone();
        }
        self.trigger_translate();
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn fetch_translation(text: &str, from: &str, to: &str) -> Result<String, String> {
    let encoded = url_encode(text);
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={from}&tl={to}&dt=t&q={encoded}"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    if let Ok(resp) = agent.get(&url).call()
        && let Ok(val) = resp.into_json::<serde_json::Value>()
        && let Some(sentences) = val.get(0).and_then(|v| v.as_array())
    {
        let mut combined = String::new();
        for seg in sentences {
            if let Some(part) = seg.get(0).and_then(|v| v.as_str()) {
                combined.push_str(part);
            }
        }
        if !combined.trim().is_empty() {
            return Ok(combined);
        }
    }

    // Fallback to MyMemory translation API
    let url2 = format!(
        "https://api.mymemory.translated.net/get?q={encoded}&langpair={from}|{to}"
    );
    let resp2 = agent
        .get(&url2)
        .call()
        .map_err(|e| format!("Network error: {e}"))?;
    let val2: serde_json::Value = resp2.into_json().map_err(|e| e.to_string())?;
    if let Some(txt) = val2
        .get("responseData")
        .and_then(|d| d.get("translatedText"))
        .and_then(|v| v.as_str())
        && !txt.trim().is_empty()
    {
        return Ok(txt.to_owned());
    }
    Err("Translation unavailable".into())
}

impl Module for Translator {
    fn id(&self) -> &'static str {
        "translator"
    }

    fn name(&self) -> &str {
        "Translator"
    }

    fn icon(&self) -> &'static str {
        icons::TRANSLATE
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn attach(&mut self, ctx: &egui::Context) {
        self.ctx = Some(ctx.clone());
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
            ui.id().with("translator-tile"),
            Sense::click(),
        );

        let (busy, preview) = self
            .state
            .lock()
            .map(|s| (s.busy, s.result.clone()))
            .unwrap_or((false, String::new()));

        match size {
            TileSize::Small => {
                let _ = kit::tool_tile(ui, ctx, icons::TRANSLATE, busy);
            }
            TileSize::Wide => {
                tile_header(ui, icons::TRANSLATE, self.name(), busy, theme);
                let pair = format!(
                    "{} → {}",
                    LANGUAGES[self.from_idx].0.to_uppercase(),
                    LANGUAGES[self.to_idx].0.to_uppercase()
                );
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 29.0),
                    Align2::LEFT_CENTER,
                    &pair,
                    Theme::font_semibold(SIZE_LABEL - 1.0),
                    theme.accent_fg(),
                );
                let short: String = preview.chars().take(14).collect();
                kit::text(
                    p,
                    pos2(rect.max.x, rect.min.y + 29.0),
                    Align2::RIGHT_CENTER,
                    if busy { "..." } else { &short },
                    Theme::font(SIZE_LABEL),
                    theme.text,
                );
            }
            TileSize::Large => {
                tile_header(ui, icons::TRANSLATE, self.name(), busy, theme);
                let p = ui.painter();
                let pair = format!(
                    "{}  ⇄  {}",
                    LANGUAGES[self.from_idx].1, LANGUAGES[self.to_idx].1
                );
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 28.0),
                    Align2::LEFT_CENTER,
                    &pair,
                    Theme::font_semibold(SIZE_LABEL),
                    theme.accent_fg(),
                );
                let src_short: String = self.input.chars().take(20).collect();
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 50.0),
                    Align2::LEFT_CENTER,
                    &src_short,
                    Theme::font(SIZE_LABEL),
                    theme.text_dim,
                );
                let dst_short: String = preview.chars().take(20).collect();
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 72.0),
                    Align2::LEFT_CENTER,
                    &dst_short,
                    Theme::font_semibold(SIZE_BODY),
                    theme.text,
                );
            }
        }

        if resp.clicked() {
            self.open_page = true;
        }
    }

    fn ui_page(&mut self, ui: &mut Ui, header: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let mut do_swap = false;
        let mut do_translate = false;
        let mut do_copy = false;

        if kit::pill(header, &nook_ui::i18n::tr("Swap"), false, theme).clicked() {
            do_swap = true;
        }
        header.add_space(6.0);
        if kit::pill(header, &nook_ui::i18n::tr("Translate"), true, theme).clicked() {
            do_translate = true;
        }
        header.add_space(6.0);
        if kit::pill(header, &nook_ui::i18n::tr("Copy Result"), false, theme).clicked() {
            do_copy = true;
        }

        if do_swap {
            self.swap_languages();
        }

        // Row 1: From / To language selector strips
        let lang_card = Rect::from_min_size(body.min, vec2(body.width(), 68.0));
        kit::card(ui.painter(), lang_card, theme);

        let bw = (lang_card.width() - 90.0 - 42.0) / LANGUAGES.len() as f32;
        for (row_idx, is_target) in [(0usize, false), (1usize, true)] {
            let y = lang_card.top() + 8.0 + row_idx as f32 * 28.0;
            let lbl = if is_target {
                nook_ui::i18n::tr("To:")
            } else {
                nook_ui::i18n::tr("From:")
            };
            kit::text(
                ui.painter(),
                pos2(lang_card.left() + 12.0, y + 12.0),
                Align2::LEFT_CENTER,
                &lbl,
                Theme::font_semibold(SIZE_LABEL),
                theme.text_dim,
            );
            for (i, &(_, name)) in LANGUAGES.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(lang_card.left() + 72.0 + i as f32 * (bw + 5.0), y),
                    vec2(bw, 23.0),
                );
                let active = if is_target {
                    self.to_idx == i
                } else {
                    self.from_idx == i
                };
                if kit::chip(ui, r, name, active, theme).clicked() {
                    if is_target {
                        self.to_idx = i;
                    } else {
                        self.from_idx = i;
                    }
                    do_translate = true;
                }
            }
        }

        // Row 2: Source input card (left) & Translated output card (right)
        let mid_top = lang_card.bottom() + 10.0;
        let half_w = (body.width() - 10.0) * 0.5;
        let src_card = Rect::from_min_size(pos2(body.left(), mid_top), vec2(half_w, 128.0));
        let dst_card = Rect::from_min_size(pos2(src_card.right() + 10.0, mid_top), vec2(half_w, 128.0));
        kit::card(ui.painter(), src_card, theme);
        kit::card(ui.painter(), dst_card, theme);

        kit::text(
            ui.painter(),
            pos2(src_card.left() + 12.0, src_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &format!("{} ({})", nook_ui::i18n::tr("Source Text"), LANGUAGES[self.from_idx].1),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );
        let field_r = Rect::from_min_size(
            pos2(src_card.left() + 10.0, src_card.top() + 32.0),
            vec2(src_card.width() - 20.0, 34.0),
        );
        let hint = nook_ui::i18n::tr("Type text to translate...");
        let resp_f = kit::field(
            ui,
            field_r,
            &mut self.input,
            &hint,
            Id::new("trans-src-field"),
            Theme::font(SIZE_BODY),
            theme,
        );
        if resp_f.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            do_translate = true;
        }

        let tr_btn = Rect::from_min_size(
            pos2(src_card.left() + 10.0, src_card.bottom() - 36.0),
            vec2(110.0, 26.0),
        );
        if kit::button(
            ui,
            tr_btn,
            &nook_ui::i18n::tr("Translate"),
            Some(icons::TRANSLATE),
            true,
            theme,
        )
        .clicked()
        {
            do_translate = true;
        }

        let clr_btn = Rect::from_min_size(
            pos2(tr_btn.right() + 8.0, src_card.bottom() - 36.0),
            vec2(78.0, 26.0),
        );
        if kit::button(
            ui,
            clr_btn,
            &nook_ui::i18n::tr("Clear"),
            Some(icons::TRASH),
            false,
            theme,
        )
        .clicked()
        {
            self.input.clear();
        }

        if do_translate {
            self.trigger_translate();
        }

        let (busy, result_txt, err_txt, history) = self
            .state
            .lock()
            .map(|s| (s.busy, s.result.clone(), s.error.clone(), s.history.clone()))
            .unwrap_or_default();

        if do_copy && !result_txt.is_empty() {
            ui.ctx().copy_text(result_txt.clone());
        }

        kit::text(
            ui.painter(),
            pos2(dst_card.left() + 12.0, dst_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &format!("{} ({})", nook_ui::i18n::tr("Translation"), LANGUAGES[self.to_idx].1),
            Theme::font_semibold(SIZE_LABEL),
            theme.accent_fg(),
        );

        let out_box = Rect::from_min_size(
            pos2(dst_card.left() + 10.0, dst_card.top() + 32.0),
            vec2(dst_card.width() - 20.0, 52.0),
        );
        ui.painter()
            .rect_filled(out_box, 8.0, with_alpha(theme.text, 0.04));
        let display_out = if busy {
            nook_ui::i18n::tr("Translating...")
        } else if let Some(e) = err_txt {
            e
        } else {
            result_txt.clone()
        };
        kit::one_line(
            ui,
            pos2(out_box.left() + 10.0, out_box.center().y - 8.0),
            &nook_ui::arabic::shape_bidi(&display_out),
            out_box.width() - 20.0,
            Theme::font_semibold(SIZE_BODY + 1.0),
            theme.text,
        );

        let cp_btn = Rect::from_min_size(
            pos2(dst_card.left() + 10.0, dst_card.bottom() - 36.0),
            vec2(124.0, 26.0),
        );
        if kit::button(
            ui,
            cp_btn,
            &nook_ui::i18n::tr("Copy Result"),
            Some(icons::COPY),
            false,
            theme,
        )
        .clicked()
            && !result_txt.is_empty()
        {
            ui.ctx().copy_text(result_txt);
        }

        // Row 3: Recent Translations History
        let hist_top = src_card.bottom() + 10.0;
        let hist_card = Rect::from_min_max(pos2(body.left(), hist_top), body.max);
        kit::card(ui.painter(), hist_card, theme);
        kit::text(
            ui.painter(),
            pos2(hist_card.left() + 12.0, hist_card.top() + 15.0),
            Align2::LEFT_CENTER,
            &nook_ui::i18n::tr("Recent Translations"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        let mut ry = hist_card.top() + 30.0;
        for (i, item) in history.iter().take(3).enumerate() {
            if ry + 24.0 > hist_card.bottom() - 4.0 {
                break;
            }
            let row_r = Rect::from_min_size(
                pos2(hist_card.left() + 10.0, ry),
                vec2(hist_card.width() - 20.0, 24.0),
            );
            let resp = kit::row(ui, row_r, Id::new(("trans-hist", i)), false, theme);
            let p = ui.painter();
            kit::text(
                p,
                pos2(row_r.left() + 8.0, row_r.center().y),
                Align2::LEFT_CENTER,
                &format!(
                    "[{}→{}]  {}",
                    item.from_code.to_uppercase(),
                    item.to_code.to_uppercase(),
                    item.source
                ),
                Theme::font(SIZE_LABEL),
                theme.text_dim,
            );
            kit::text(
                p,
                pos2(row_r.right() - 8.0, row_r.center().y),
                Align2::RIGHT_CENTER,
                &item.translated,
                Theme::font_semibold(SIZE_LABEL),
                theme.text,
            );
            if resp.clicked() {
                ui.ctx().copy_text(item.translated.clone());
            }
            ry += 26.0;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encoding_preserves_ascii_and_escapes_spaces() {
        assert_eq!(url_encode("Hello World"), "Hello%20World");
    }
}
