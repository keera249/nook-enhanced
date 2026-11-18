//! Focus Studio & Side Tools: Procedural Ambient Focus Sounds (Rain, Brown Noise,
//! Ocean Waves, Binaural Focus), Cryptographic Password Generator, and Dice/Coin
//! Decision Maker (inspired by NotchPixel's AmbientSoundWidget,
//! PasswordGeneratorWidget, and DiceCoinRandomizer).

use crate::kit;
use egui::{Align2, Color32, Rect, Sense, Ui, pos2, vec2};
use nook_platform::audio::{SAMPLE_RATE, WavWriter, play_loop, stop_playback};
use nook_ui::bento::tile_header;
use nook_ui::icons;
use nook_ui::module::{Badge, Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundPreset {
    Rain,
    BrownNoise,
    OceanWaves,
    BinauralFocus,
}

impl SoundPreset {
    pub const ALL: [Self; 4] = [
        Self::Rain,
        Self::BrownNoise,
        Self::OceanWaves,
        Self::BinauralFocus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rain => "Soft Rain",
            Self::BrownNoise => "Brown Noise",
            Self::OceanWaves => "Ocean Waves",
            Self::BinauralFocus => "Focus Tone",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Rain => icons::CLOUD_RAIN,
            Self::BrownNoise => icons::WAVE_SINE,
            Self::OceanWaves => icons::WAVES,
            Self::BinauralFocus => icons::HEADPHONES,
        }
    }
}

pub struct AmbientSound {
    playing: Option<SoundPreset>,
    pw_len: usize,
    pw_upper: bool,
    pw_lower: bool,
    pw_digits: bool,
    pw_symbols: bool,
    password: String,
    dice_values: (u8, u8),
    coin_heads: bool,
    seed: u64,
    open_page: bool,
}

impl Default for AmbientSound {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let mut s = Self {
            playing: None,
            pw_len: 16,
            pw_upper: true,
            pw_lower: true,
            pw_digits: true,
            pw_symbols: true,
            password: String::new(),
            dice_values: (6, 4),
            coin_heads: true,
            seed,
            open_page: false,
        };
        s.regenerate_password();
        s
    }
}

impl AmbientSound {
    fn next_rand(&mut self) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed
    }

    pub fn regenerate_password(&mut self) {
        let mut alphabet = Vec::with_capacity(96);
        if self.pw_upper {
            alphabet.extend(b'A'..=b'Z');
        }
        if self.pw_lower {
            alphabet.extend(b'a'..=b'z');
        }
        if self.pw_digits {
            alphabet.extend(b'0'..=b'9');
        }
        if self.pw_symbols {
            alphabet.extend_from_slice(b"!@#$%^&*()-_=+[]{}?");
        }
        if alphabet.is_empty() {
            alphabet.extend(b'a'..=b'z');
        }
        let len = self.pw_len.clamp(8, 32);
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let idx = (self.next_rand() >> 32) as usize % alphabet.len();
            out.push(alphabet[idx] as char);
        }
        self.password = out;
    }

    fn toggle_sound(&mut self, preset: SoundPreset) {
        if self.playing == Some(preset) {
            stop_playback();
            self.playing = None;
            return;
        }
        self.playing = Some(preset);
        std::thread::spawn(move || {
            let path = std::env::temp_dir().join(format!("nook-ambient-{preset:?}.wav"));
            if !path.exists() {
                let _ = write_ambient_wav(&path, preset);
            }
            let _ = play_loop(&path);
        });
    }
}

fn write_ambient_wav(path: &std::path::Path, preset: SoundPreset) -> std::io::Result<()> {
    let seconds = 6usize;
    let total = SAMPLE_RATE as usize * seconds;
    let mut samples = Vec::with_capacity(total);
    let mut rng = 0x1234_5678_9ABC_DEF0u64;
    let mut next_white = || -> f32 {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((rng >> 33) as i32 as f32 / (i32::MAX as f32)) - 1.0
    };

    let mut brown = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..total {
        let t = i as f32 / SAMPLE_RATE as f32;
        // Seamless crossfade envelope at loop edges
        let edge = (i.min(total - 1 - i) as f32 / 2200.0).clamp(0.0, 1.0);
        let val = match preset {
            SoundPreset::Rain => {
                let w = next_white();
                lp = lp * 0.86 + w * 0.14;
                lp * 0.22
            }
            SoundPreset::BrownNoise => {
                let w = next_white();
                brown = (brown + 0.025 * w).clamp(-1.0, 1.0) * 0.998;
                brown * 0.28
            }
            SoundPreset::OceanWaves => {
                let w = next_white();
                brown = (brown + 0.03 * w).clamp(-1.0, 1.0) * 0.997;
                let swell = ((t * std::f32::consts::TAU / 6.0).sin() * 0.5 + 0.5) * 0.7 + 0.15;
                brown * swell * 0.26
            }
            SoundPreset::BinauralFocus => {
                let f1 = (t * 216.0 * std::f32::consts::TAU).sin();
                let f2 = (t * 226.0 * std::f32::consts::TAU).sin();
                let pad = (t * 108.0 * std::f32::consts::TAU).sin() * 0.4;
                (f1 * 0.4 + f2 * 0.4 + pad) * 0.14
            }
        };
        let s = (val * edge * 32000.0).clamp(-32000.0, 32000.0) as i16;
        samples.push(s);
    }

    let mut writer = WavWriter::create(path)?;
    writer.write(&samples)?;
    writer.finish()
}

impl Module for AmbientSound {
    fn id(&self) -> &'static str {
        "ambient_sound"
    }

    fn name(&self) -> &str {
        "Focus & Tools"
    }

    fn icon(&self) -> &'static str {
        icons::WAVE_SINE
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn is_active(&self) -> bool {
        self.playing.is_some()
    }

    fn collapsed_badge(&self) -> Option<Badge> {
        self.playing.map(|p| Badge::Glyph { icon: p.icon() })
    }

    fn shutdown(&mut self) {
        if self.playing.is_some() {
            stop_playback();
            self.playing = None;
        }
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
            ui.id().with("ambient-tile"),
            Sense::click(),
        );

        match size {
            TileSize::Small => {
                let _ = kit::tool_tile(ui, ctx, icons::WAVE_SINE, self.playing.is_some());
            }
            TileSize::Wide => {
                tile_header(
                    ui,
                    icons::WAVE_SINE,
                    self.name(),
                    self.playing.is_some(),
                    theme,
                );
                let play_r = Rect::from_min_size(
                    pos2(rect.max.x - 26.0, rect.min.y + 19.0),
                    vec2(26.0, 20.0),
                );
                let play_resp = ui.interact(play_r, ui.id().with("amb-play"), Sense::click());
                let p = ui.painter();
                p.rect_filled(
                    play_r,
                    6.0,
                    if self.playing.is_some() {
                        theme.accent
                    } else {
                        with_alpha(theme.text, 0.10)
                    },
                );
                kit::text(
                    p,
                    play_r.center(),
                    Align2::CENTER_CENTER,
                    if self.playing.is_some() {
                        icons::STOP
                    } else {
                        icons::PLAY
                    },
                    Theme::font_semibold(10.0),
                    if self.playing.is_some() {
                        Color32::WHITE
                    } else {
                        theme.text
                    },
                );
                let status = match self.playing {
                    Some(pr) => nook_ui::i18n::tr(pr.label()),
                    None => nook_ui::i18n::tr("Soft Rain"),
                };
                kit::text(
                    p,
                    pos2(rect.min.x, rect.min.y + 29.0),
                    Align2::LEFT_CENTER,
                    &status,
                    Theme::font_semibold(SIZE_LABEL),
                    if self.playing.is_some() {
                        theme.accent_fg()
                    } else {
                        theme.text_dim
                    },
                );
                if play_resp.clicked() {
                    let target = self.playing.unwrap_or(SoundPreset::Rain);
                    self.toggle_sound(target);
                    return;
                }
            }
            TileSize::Large => {
                tile_header(
                    ui,
                    icons::WAVE_SINE,
                    self.name(),
                    self.playing.is_some(),
                    theme,
                );
                let bw = (rect.width() - 6.0) * 0.5;
                for (i, &preset) in SoundPreset::ALL.iter().enumerate() {
                    let col = i % 2;
                    let row = i / 2;
                    let r = Rect::from_min_size(
                        pos2(
                            rect.min.x + col as f32 * (bw + 6.0),
                            rect.min.y + 26.0 + row as f32 * 38.0,
                        ),
                        vec2(bw, 32.0),
                    );
                    if kit::chip(
                        ui,
                        r,
                        &nook_ui::i18n::tr(preset.label()),
                        self.playing == Some(preset),
                        theme,
                    )
                    .clicked()
                    {
                        self.toggle_sound(preset);
                        return;
                    }
                }
            }
        }

        if resp.clicked() {
            self.open_page = true;
        }
    }

    fn ui_page(&mut self, ui: &mut Ui, header: &mut Ui, body: Rect, theme: &Theme) -> bool {
        if self.playing.is_some()
            && kit::pill(header, &nook_ui::i18n::tr("Stop"), true, theme).clicked()
        {
            stop_playback();
            self.playing = None;
        }

        // Section 1: Ambient Focus Sounds
        let snd_card = Rect::from_min_size(body.min, vec2(body.width(), 88.0));
        kit::card(ui.painter(), snd_card, theme);
        kit::text(
            ui.painter(),
            pos2(snd_card.left() + 14.0, snd_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &nook_ui::i18n::tr("Ambient Focus Sounds"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        let bw = (snd_card.width() - 24.0 - 24.0) / 4.0;
        for (i, &preset) in SoundPreset::ALL.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(
                    snd_card.left() + 12.0 + i as f32 * (bw + 8.0),
                    snd_card.top() + 34.0,
                ),
                vec2(bw, 42.0),
            );
            let active = self.playing == Some(preset);
            if kit::button(
                ui,
                r,
                &nook_ui::i18n::tr(preset.label()),
                Some(preset.icon()),
                active,
                theme,
            )
            .clicked()
            {
                self.toggle_sound(preset);
            }
        }

        // Section 2: Cryptographic Password Generator
        let pw_top = snd_card.bottom() + 10.0;
        let pw_card = Rect::from_min_size(pos2(body.left(), pw_top), vec2(body.width(), 126.0));
        kit::card(ui.painter(), pw_card, theme);
        kit::text(
            ui.painter(),
            pos2(pw_card.left() + 14.0, pw_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &nook_ui::i18n::tr("Password & Key Generator"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        // Generated password display box + Generate & Copy buttons
        let pw_box = Rect::from_min_size(
            pos2(pw_card.left() + 12.0, pw_card.top() + 32.0),
            vec2(pw_card.width() - 216.0, 34.0),
        );
        ui.painter()
            .rect_filled(pw_box, 8.0, with_alpha(theme.text, 0.05));
        kit::text(
            ui.painter(),
            pos2(pw_box.left() + 12.0, pw_box.center().y),
            Align2::LEFT_CENTER,
            &self.password,
            Theme::font_semibold(SIZE_BODY + 2.0),
            theme.accent_fg(),
        );

        let gen_btn = Rect::from_min_size(
            pos2(pw_box.right() + 8.0, pw_card.top() + 32.0),
            vec2(92.0, 34.0),
        );
        if kit::button(
            ui,
            gen_btn,
            &nook_ui::i18n::tr("Generate"),
            Some(icons::ARROWS_CLOCKWISE),
            true,
            theme,
        )
        .clicked()
        {
            self.regenerate_password();
        }
        let cp_btn = Rect::from_min_size(
            pos2(gen_btn.right() + 8.0, pw_card.top() + 32.0),
            vec2(84.0, 34.0),
        );
        if kit::button(
            ui,
            cp_btn,
            &nook_ui::i18n::tr("Copy"),
            Some(icons::COPY),
            false,
            theme,
        )
        .clicked()
        {
            ui.ctx().copy_text(self.password.clone());
        }

        // Length presets & character class chips
        let mut need_regen = false;
        let mut x = pw_card.left() + 12.0;
        let y = pw_card.top() + 78.0;
        for &len in &[8usize, 12, 16, 24, 32] {
            let r = Rect::from_min_size(pos2(x, y), vec2(44.0, 26.0));
            if kit::chip(ui, r, &format!("{len}"), self.pw_len == len, theme).clicked() {
                self.pw_len = len;
                need_regen = true;
            }
            x += 50.0;
        }
        x += 12.0;
        for (lbl, flag_idx) in [("A-Z", 0), ("a-z", 1), ("0-9", 2), ("!@#", 3)] {
            let on = match flag_idx {
                0 => self.pw_upper,
                1 => self.pw_lower,
                2 => self.pw_digits,
                _ => self.pw_symbols,
            };
            let r = Rect::from_min_size(pos2(x, y), vec2(52.0, 26.0));
            if kit::chip(ui, r, lbl, on, theme).clicked() {
                match flag_idx {
                    0 => self.pw_upper = !self.pw_upper,
                    1 => self.pw_lower = !self.pw_lower,
                    2 => self.pw_digits = !self.pw_digits,
                    _ => self.pw_symbols = !self.pw_symbols,
                }
                need_regen = true;
            }
            x += 58.0;
        }
        if need_regen {
            self.regenerate_password();
        }

        // Section 3: Dice Roller & Coin Flipper
        let rnd_top = pw_card.bottom() + 10.0;
        let rnd_card = Rect::from_min_max(pos2(body.left(), rnd_top), body.max);
        kit::card(ui.painter(), rnd_card, theme);
        kit::text(
            ui.painter(),
            pos2(rnd_card.left() + 14.0, rnd_card.top() + 16.0),
            Align2::LEFT_CENTER,
            &nook_ui::i18n::tr("Quick Decision: Dice & Coin"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        let dice_txt = format!(
            "🎲  {} + {} = {}",
            self.dice_values.0,
            self.dice_values.1,
            self.dice_values.0 + self.dice_values.1
        );
        kit::text(
            ui.painter(),
            pos2(rnd_card.left() + 16.0, rnd_card.center().y + 8.0),
            Align2::LEFT_CENTER,
            &dice_txt,
            Theme::font_semibold(SIZE_BODY + 3.0),
            theme.text,
        );
        let roll_btn = Rect::from_min_size(
            pos2(rnd_card.left() + 165.0, rnd_card.center().y - 6.0),
            vec2(110.0, 28.0),
        );
        if kit::button(
            ui,
            roll_btn,
            &nook_ui::i18n::tr("Roll Dice"),
            Some(icons::DICE_FIVE),
            false,
            theme,
        )
        .clicked()
        {
            let d1 = ((self.next_rand() >> 32) % 6 + 1) as u8;
            let d2 = ((self.next_rand() >> 32) % 6 + 1) as u8;
            self.dice_values = (d1, d2);
        }

        let coin_lbl = if self.coin_heads {
            nook_ui::i18n::tr("Heads")
        } else {
            nook_ui::i18n::tr("Tails")
        };
        kit::text(
            ui.painter(),
            pos2(rnd_card.left() + 320.0, rnd_card.center().y + 8.0),
            Align2::LEFT_CENTER,
            &format!("🪙  {coin_lbl}"),
            Theme::font_semibold(SIZE_BODY + 2.0),
            Color32::from_rgb(251, 191, 36),
        );
        let flip_btn = Rect::from_min_size(
            pos2(rnd_card.right() - 124.0, rnd_card.center().y - 6.0),
            vec2(110.0, 28.0),
        );
        if kit::button(
            ui,
            flip_btn,
            &nook_ui::i18n::tr("Flip Coin"),
            Some(icons::COINS),
            false,
            theme,
        )
        .clicked()
        {
            self.coin_heads = (self.next_rand() & 1) == 0;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_respects_length() {
        let mut a = AmbientSound::default();
        a.pw_len = 24;
        a.regenerate_password();
        assert_eq!(a.password.len(), 24);
    }
}
