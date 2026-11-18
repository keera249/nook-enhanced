//! Quick Control Center: Screen Brightness slider, Microphone Mute switch
//! (with `Ctrl+Alt+M` shortcut), Volume & Mute controls, Night Light toggle,
//! live Privacy indicator (Mic/Cam), and one-click Windows Settings tiles
//! (inspired by NotchPixel's ControlTiles, BrightnessService & MicrophoneService).

use crate::kit;
use egui::{Align2, Color32, Rect, Sense, Ui, pos2, vec2};
use nook_platform::{shell, system};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::{icons, widgets};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug)]
struct ControlState {
    brightness: u8,
    mic_muted: bool,
    night_light: bool,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            brightness: 80,
            mic_muted: false,
            night_light: false,
        }
    }
}

pub struct ControlCenter {
    state: Arc<Mutex<ControlState>>,
    brightness_ui: u8,
    last_sync: Option<Instant>,
    open_page: bool,
}

impl Default for ControlCenter {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ControlState::default())),
            brightness_ui: 80,
            last_sync: None,
            open_page: false,
        }
    }
}

impl ControlCenter {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.query_brightness_async();
        s
    }

    fn query_brightness_async(&mut self) {
        self.last_sync = Some(Instant::now());
        let state_arc = Arc::clone(&self.state);
        std::thread::spawn(move || {
            #[cfg(windows)]
            let val_opt = {
                let out = Command::new("powershell")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness",
                    ])
                    .output();
                out.ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|t| t.trim().parse::<u8>().ok())
            };
            #[cfg(not(windows))]
            let val_opt = {
                Command::new("brightnessctl")
                    .args(["-m"])
                    .output()
                    .ok()
                    .and_then(|o| {
                        let s = String::from_utf8_lossy(&o.stdout);
                        s.split(',')
                            .nth(3)
                            .and_then(|p| p.trim_end_matches('%').parse::<u8>().ok())
                    })
            };
            if let Some(val) = val_opt
                && let Ok(mut st) = state_arc.lock()
            {
                st.brightness = val.min(100);
            }
        });
    }

    fn set_brightness_async(&mut self, level: u8) {
        let clamped = level.min(100);
        self.brightness_ui = clamped;
        if let Ok(mut st) = self.state.lock() {
            st.brightness = clamped;
        }
        std::thread::spawn(move || {
            #[cfg(windows)]
            {
                let cmd = format!(
                    "$m = Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue; if ($m) {{ $m.WmiSetBrightness(1, {clamped}) }}"
                );
                let _ = Command::new("powershell")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
                    .status();
            }
            #[cfg(not(windows))]
            {
                let _ = Command::new("brightnessctl")
                    .args(["set", &format!("{clamped}%")])
                    .status();
            }
        });
    }

    fn toggle_mic_mute(&mut self) {
        let muted = if let Ok(mut st) = self.state.lock() {
            st.mic_muted = !st.mic_muted;
            st.mic_muted
        } else {
            false
        };
        std::thread::spawn(move || {
            let _ = muted;
            #[cfg(windows)]
            {
                let cmd = r#"Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class K { [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h, int m, IntPtr w, IntPtr l); }'; $h = [K]::GetForegroundWindow(); [K]::SendMessageW($h, 0x0319, $h, [IntPtr](24 -shl 16)) | Out-Null"#;
                let _ = Command::new("powershell")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
                    .status();
            }
            #[cfg(not(windows))]
            {
                let _ = Command::new("pactl")
                    .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
                    .status();
            }
        });
    }

    fn send_volume_key(vk: u8) {
        match vk {
            0xAE => system::volume_step(-1),
            0xAD => system::volume_mute_toggle(),
            0xAF => system::volume_step(1),
            _ => {}
        }
    }
}

impl Module for ControlCenter {
    fn id(&self) -> &'static str {
        "control_center"
    }

    fn name(&self) -> &str {
        "Control Center"
    }

    fn icon(&self) -> &'static str {
        icons::SLIDERS_HORIZONTAL
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn default_hotkey(&self) -> Option<&'static str> {
        Some("ctrl+alt+KeyM")
    }

    fn on_hotkey(&mut self) {
        self.toggle_mic_mute();
        system::alert_sound();
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        if self.last_sync.is_none() {
            self.query_brightness_async();
        }
        let st = self.state.lock().map(|s| s.clone()).unwrap_or_default();
        self.brightness_ui = st.brightness;
        let (mic_live, cam_live) = system::privacy_in_use();
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::SLIDERS_HORIZONTAL, "Controls", !st.mic_muted, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &format!("{} {}%", icons::SUN, st.brightness),
                    Theme::font_semibold(14.5),
                    theme.text,
                );
                let mic_lbl = if st.mic_muted { "Mic Muted" } else { "Mic Ready" };
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &nook_ui::i18n::tr(mic_lbl),
                    Theme::font(10.0),
                    if st.mic_muted {
                        Color32::from_rgb(245, 110, 110)
                    } else {
                        theme.text_dim
                    },
                );
                if ui
                    .interact(rect, ui.id().with("cc-sm"), Sense::click())
                    .clicked()
                {
                    self.open_page = true;
                }
            }
            TileSize::Wide => {
                tile_header(ui, icons::SLIDERS_HORIZONTAL, "Control Center", false, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &format!("{} {}%", icons::SUN, st.brightness),
                    Theme::font_semibold(11.5),
                    theme.text,
                );
                let bar_r = Rect::from_min_size(
                    pos2(content.left() + 46.0, y - 3.5),
                    vec2((content.width() - 46.0).max(24.0), 7.0),
                );
                kit::bar(
                    p,
                    bar_r,
                    f32::from(st.brightness) / 100.0,
                    theme.accent_fg(),
                    theme,
                );
                let resp = ui.interact(bar_r.expand(4.0), ui.id().with("cc-w-br"), Sense::click());
                if resp.clicked()
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    let pct = (((pos.x - bar_r.left()) / bar_r.width()).clamp(0.05, 1.0) * 100.0)
                        .round() as u8;
                    self.set_brightness_async(pct);
                } else {
                    let open_r = Rect::from_min_size(rect.min, vec2(rect.width(), 28.0));
                    if ui
                        .interact(open_r, ui.id().with("cc-w-open"), Sense::click())
                        .clicked()
                    {
                        self.open_page = true;
                    }
                }
            }
            TileSize::Large => {
                tile_header(ui, icons::SLIDERS_HORIZONTAL, "Control Center", false, theme);
                let p = ui.painter();

                // Brightness slider bar
                let br_y = content.top() + 22.0;
                kit::text(
                    p,
                    pos2(content.left(), br_y),
                    Align2::LEFT_TOP,
                    &format!("{} {}", icons::SUN, nook_ui::i18n::tr("Brightness")),
                    Theme::font(10.0),
                    theme.text_dim,
                );
                kit::text(
                    p,
                    pos2(content.right(), br_y),
                    Align2::RIGHT_TOP,
                    &format!("{}%", st.brightness),
                    Theme::font_semibold(10.5),
                    theme.text,
                );
                let bar_r = Rect::from_min_size(
                    pos2(content.left(), br_y + 14.0),
                    vec2(content.width(), 7.0),
                );
                kit::bar(
                    p,
                    bar_r,
                    f32::from(st.brightness) / 100.0,
                    theme.accent_fg(),
                    theme,
                );
                let br_resp =
                    ui.interact(bar_r.expand(4.0), ui.id().with("cc-lg-br"), Sense::click());
                if br_resp.clicked()
                    && let Some(pos) = br_resp.interact_pointer_pos()
                {
                    let pct = (((pos.x - bar_r.left()) / bar_r.width()).clamp(0.05, 1.0) * 100.0)
                        .round() as u8;
                    self.set_brightness_async(pct);
                }

                // 2x2 Quick toggle pills
                let grid_top = br_y + 26.0;
                let col_w = (content.width() - 6.0) * 0.5;
                let row_h = 25.0;
                let tiles = [
                    (
                        0,
                        0,
                        if st.mic_muted {
                            icons::MICROPHONE_SLASH
                        } else {
                            icons::MICROPHONE
                        },
                        if st.mic_muted { "Muted" } else { "Mic On" },
                        !st.mic_muted,
                    ),
                    (1, 0, icons::MOON, "Night Light", st.night_light),
                    (0, 1, icons::WIFI_HIGH, "Wi-Fi", true),
                    (1, 1, icons::BLUETOOTH, "Bluetooth", true),
                ];

                let mut clicked_idx = None;
                for (idx, &(c, r, ic, lbl, active)) in tiles.iter().enumerate() {
                    let rx = content.left() + c as f32 * (col_w + 6.0);
                    let ry = grid_top + r as f32 * (row_h + 5.0);
                    let t_rect = Rect::from_min_size(pos2(rx, ry), vec2(col_w, row_h));
                    let resp = ui.interact(t_rect, ui.id().with(("cc-q", idx)), Sense::click());
                    let p = ui.painter();
                    let fill = if active {
                        with_alpha(theme.accent, if resp.hovered() { 0.42 } else { 0.28 })
                    } else if resp.hovered() {
                        theme.tile_top
                    } else {
                        theme.tile_bottom
                    };
                    p.rect_filled(t_rect, 7.0, fill);
                    kit::text(
                        p,
                        pos2(t_rect.left() + 6.0, t_rect.center().y),
                        Align2::LEFT_CENTER,
                        &format!("{ic} {}", nook_ui::i18n::tr(lbl)),
                        Theme::font_semibold(9.5),
                        if active {
                            theme.accent_fg()
                        } else {
                            theme.text_dim
                        },
                    );
                    if resp.clicked() {
                        clicked_idx = Some(idx);
                    }
                }

                // Privacy footer
                let p = ui.painter();
                let priv_text = if mic_live || cam_live {
                    nook_ui::i18n::tr("Privacy: Mic/Cam Active")
                } else {
                    nook_ui::i18n::tr("Privacy: Protected")
                };
                let priv_col = if mic_live || cam_live {
                    Color32::from_rgb(255, 165, 50)
                } else {
                    Color32::from_rgb(90, 215, 130)
                };
                p.circle_filled(
                    pos2(content.left() + 4.0, content.bottom() - 4.0),
                    3.5,
                    priv_col,
                );
                kit::text(
                    p,
                    pos2(content.left() + 11.0, content.bottom() + 1.0),
                    Align2::LEFT_BOTTOM,
                    &priv_text,
                    Theme::font(9.5),
                    theme.text_dim,
                );

                match clicked_idx {
                    Some(0) => self.toggle_mic_mute(),
                    Some(1) => {
                        if let Ok(mut s) = self.state.lock() {
                            s.night_light = !s.night_light;
                        }
                        let _ = shell::open("ms-settings:nightlight");
                    }
                    Some(2) => {
                        let _ = shell::open("ms-settings:network-wifi");
                    }
                    Some(3) => {
                        let _ = shell::open("ms-settings:bluetooth");
                    }
                    _ => {
                        let hdr = Rect::from_min_size(rect.min, vec2(rect.width(), 28.0));
                        if ui
                            .interact(hdr, ui.id().with("cc-lg-open"), Sense::click())
                            .clicked()
                        {
                            self.open_page = true;
                        }
                    }
                }
            }
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 255.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let st = self.state.lock().map(|s| s.clone()).unwrap_or_default();
        let (mic_live, cam_live) = system::privacy_in_use();
        let mut new_brightness = None;
        let mut toggle_mic = false;

        if widgets::icon_button(actions, icons::ARROWS_CLOCKWISE, 24.0, false, theme).clicked() {
            self.query_brightness_async();
        }

        let p = ui.painter();
        let left_w = 278.0;
        let br_card = Rect::from_min_size(pos2(body.left(), body.top()), vec2(left_w, 100.0));
        let aud_card = Rect::from_min_max(
            pos2(br_card.right() + 10.0, body.top()),
            pos2(body.right(), body.top() + 100.0),
        );
        kit::card(p, br_card, theme);
        kit::card(p, aud_card, theme);

        // Brightness card
        kit::text(
            p,
            pos2(br_card.left() + 12.0, br_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::SUN, nook_ui::i18n::tr("Screen Brightness")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );
        kit::text(
            p,
            pos2(br_card.right() - 12.0, br_card.top() + 10.0),
            Align2::RIGHT_TOP,
            &format!("{}%", st.brightness),
            Theme::font_semibold(SIZE_BODY),
            theme.accent_fg(),
        );
        let slider_r = Rect::from_min_size(
            pos2(br_card.left() + 12.0, br_card.top() + 38.0),
            vec2(br_card.width() - 24.0, 12.0),
        );
        kit::bar(
            p,
            slider_r,
            f32::from(st.brightness) / 100.0,
            theme.accent_fg(),
            theme,
        );
        let s_resp = ui.interact(slider_r.expand(4.0), ui.id().with("cc-pg-br"), Sense::click());
        if s_resp.clicked()
            && let Some(pos) = s_resp.interact_pointer_pos()
        {
            let pct = (((pos.x - slider_r.left()) / slider_r.width()).clamp(0.05, 1.0) * 100.0)
                .round() as u8;
            new_brightness = Some(pct);
        }

        for (i, &preset) in [25u8, 50, 75, 100].iter().enumerate() {
            let pw = (br_card.width() - 24.0 - 18.0) / 4.0;
            let pr = Rect::from_min_size(
                pos2(br_card.left() + 12.0 + i as f32 * (pw + 6.0), br_card.top() + 62.0),
                vec2(pw, 24.0),
            );
            let resp = ui.interact(pr, ui.id().with(("br-pre", preset)), Sense::click());
            let active = (i16::from(st.brightness) - i16::from(preset)).abs() <= 5;
            let p = ui.painter();
            p.rect_filled(
                pr,
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
                pr.center(),
                Align2::CENTER_CENTER,
                &format!("{preset}%"),
                Theme::font_semibold(10.5),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() {
                new_brightness = Some(preset);
            }
        }

        // Audio & Microphone card
        let p = ui.painter();
        kit::text(
            p,
            pos2(aud_card.left() + 12.0, aud_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::SPEAKER_HIGH, nook_ui::i18n::tr("Audio & Microphone")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );
        let vol_btns: [(&str, u8); 3] = [
            ("Vol -", 0xAE),
            ("Mute", 0xAD),
            ("Vol +", 0xAF),
        ];
        let bw = (aud_card.width() - 24.0 - 12.0) / 3.0;
        for (i, &(lbl, vk)) in vol_btns.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(aud_card.left() + 12.0 + i as f32 * (bw + 6.0), aud_card.top() + 34.0),
                vec2(bw, 26.0),
            );
            let resp = ui.interact(r, ui.id().with(("cc-vol", i)), Sense::click());
            let p = ui.painter();
            p.rect_filled(
                r,
                6.0,
                if resp.hovered() {
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
                Theme::font_semibold(11.0),
                theme.text,
            );
            if resp.clicked() {
                Self::send_volume_key(vk);
            }
        }

        let mic_r = Rect::from_min_size(
            pos2(aud_card.left() + 12.0, aud_card.top() + 66.0),
            vec2(aud_card.width() - 24.0, 24.0),
        );
        let mic_resp = ui.interact(mic_r, ui.id().with("cc-pg-mic"), Sense::click());
        let p = ui.painter();
        p.rect_filled(
            mic_r,
            6.0,
            if st.mic_muted {
                Color32::from_rgba_unmultiplied(235, 85, 85, 70)
            } else {
                with_alpha(theme.accent, 0.28)
            },
        );
        let mic_txt = if st.mic_muted {
            format!(
                "{}  {}",
                icons::MICROPHONE_SLASH,
                nook_ui::i18n::tr("Microphone Muted (Ctrl+Alt+M)")
            )
        } else {
            format!(
                "{}  {}",
                icons::MICROPHONE,
                nook_ui::i18n::tr("Microphone Active (Ctrl+Alt+M)")
            )
        };
        kit::text(
            p,
            mic_r.center(),
            Align2::CENTER_CENTER,
            &mic_txt,
            Theme::font_semibold(11.0),
            theme.text,
        );
        if mic_resp.clicked() {
            toggle_mic = true;
        }

        // Bottom card: Quick Windows Settings + Privacy Monitor
        let bot_card = Rect::from_min_max(
            pos2(body.left(), body.top() + 110.0),
            pos2(body.right(), body.bottom()),
        );
        kit::card(p, bot_card, theme);
        kit::text(
            p,
            pos2(bot_card.left() + 12.0, bot_card.top() + 10.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("Quick System Shortcuts & Privacy"),
            Theme::font_semibold(SIZE_LABEL),
            theme.text_dim,
        );

        let shortcuts = [
            (icons::WIFI_HIGH, "Wi-Fi", "ms-settings:network-wifi"),
            (icons::BLUETOOTH, "Bluetooth", "ms-settings:bluetooth"),
            (icons::MOON, "Night Light", "ms-settings:nightlight"),
            (icons::SPEAKER_HIGH, "Sound", "ms-settings:sound"),
            (icons::MONITOR, "Display", "ms-settings:display"),
        ];
        let sw = (bot_card.width() - 24.0 - 24.0) / 5.0;
        for (i, &(ic, name, uri)) in shortcuts.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(bot_card.left() + 12.0 + i as f32 * (sw + 6.0), bot_card.top() + 30.0),
                vec2(sw, 34.0),
            );
            let resp = ui.interact(r, ui.id().with(("cc-sc", i)), Sense::click());
            let p = ui.painter();
            p.rect_filled(
                r,
                8.0,
                if resp.hovered() {
                    with_alpha(theme.accent, 0.30)
                } else {
                    theme.tile_bottom
                },
            );
            kit::text(
                p,
                r.center(),
                Align2::CENTER_CENTER,
                &format!("{ic}  {}", nook_ui::i18n::tr(name)),
                Theme::font_semibold(11.0),
                theme.text,
            );
            if resp.clicked() {
                let _ = shell::open(uri);
            }
        }

        let p = ui.painter();
        let priv_msg = match (mic_live, cam_live) {
            (true, true) => nook_ui::i18n::tr("Microphone & Camera are currently in use"),
            (true, false) => nook_ui::i18n::tr("Microphone is currently in use by an app"),
            (false, true) => nook_ui::i18n::tr("Camera is currently in use by an app"),
            (false, false) => nook_ui::i18n::tr("No apps are currently using Microphone or Camera"),
        };
        let dot_col = if mic_live || cam_live {
            Color32::from_rgb(255, 165, 50)
        } else {
            Color32::from_rgb(90, 215, 130)
        };
        p.circle_filled(
            pos2(bot_card.left() + 18.0, bot_card.bottom() - 14.0),
            4.5,
            dot_col,
        );
        kit::text(
            p,
            pos2(bot_card.left() + 28.0, bot_card.bottom() - 14.0),
            Align2::LEFT_CENTER,
            &priv_msg,
            Theme::font(SIZE_LABEL),
            theme.text_dim,
        );

        if let Some(b) = new_brightness {
            self.set_brightness_async(b);
        }
        if toggle_mic {
            self.toggle_mic_mute();
        }
        false
    }
}
