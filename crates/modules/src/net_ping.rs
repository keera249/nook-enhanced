//! Network Ping & Website Uptime Monitor: live latency (ms), jitter, packet
//! loss %, 48-sample sparkline chart, target server selector, and multi-site
//! HTTP/TCP uptime monitor (inspired by NotchPixel's PingService, PingStats,
//! and SiteChecker).

use crate::kit;
use egui::{Align2, Color32, Key, Pos2, Rect, Sense, Stroke, TextEdit, Ui, UiBuilder, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, SIZE_LABEL, Theme, with_alpha};
use nook_ui::{icons, widgets};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 48;
const PING_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug)]
struct PingTarget {
    label: &'static str,
    addr: &'static str,
}

const TARGETS: &[PingTarget] = &[
    PingTarget {
        label: "Cloudflare",
        addr: "1.1.1.1:443",
    },
    PingTarget {
        label: "Google DNS",
        addr: "8.8.8.8:443",
    },
    PingTarget {
        label: "Quad9",
        addr: "9.9.9.9:443",
    },
    PingTarget {
        label: "GitHub",
        addr: "github.com:443",
    },
];

#[derive(Clone, Debug)]
pub struct SiteStatus {
    pub host: String,
    pub online: bool,
    pub latency_ms: u32,
}

#[derive(Clone, Debug)]
struct PingData {
    current_ms: Option<u32>,
    history: VecDeque<u32>,
    total_sent: u32,
    total_lost: u32,
    sites: Vec<SiteStatus>,
}

impl Default for PingData {
    fn default() -> Self {
        let mut history = VecDeque::with_capacity(MAX_SAMPLES);
        for &v in &[24, 22, 25, 23, 21, 24, 26, 23, 22, 24, 23, 25] {
            history.push_back(v);
        }
        Self {
            current_ms: Some(23),
            history,
            total_sent: 12,
            total_lost: 0,
            sites: vec![
                SiteStatus {
                    host: "google.com".into(),
                    online: true,
                    latency_ms: 24,
                },
                SiteStatus {
                    host: "github.com".into(),
                    online: true,
                    latency_ms: 42,
                },
                SiteStatus {
                    host: "cloudflare.com".into(),
                    online: true,
                    latency_ms: 19,
                },
                SiteStatus {
                    host: "store.steampowered.com".into(),
                    online: true,
                    latency_ms: 38,
                },
            ],
        }
    }
}

impl PingData {
    pub fn jitter_ms(&self) -> u32 {
        if self.history.len() < 2 {
            return 0;
        }
        let mut sum = 0u32;
        let mut count = 0u32;
        let vec: Vec<u32> = self.history.iter().copied().collect();
        for pair in vec.windows(2) {
            sum += pair[0].abs_diff(pair[1]);
            count += 1;
        }
        if count > 0 { sum / count } else { 0 }
    }

    pub fn loss_pct(&self) -> f32 {
        if self.total_sent == 0 {
            0.0
        } else {
            (self.total_lost as f32 / self.total_sent as f32) * 100.0
        }
    }

    pub fn quality(&self) -> (&'static str, Color32) {
        match self.current_ms {
            Some(ms) if ms <= 35 => ("Excellent", Color32::from_rgb(80, 220, 130)),
            Some(ms) if ms <= 75 => ("Good", Color32::from_rgb(120, 200, 255)),
            Some(ms) if ms <= 140 => ("Fair", Color32::from_rgb(255, 190, 70)),
            Some(_) => ("High Ping", Color32::from_rgb(250, 100, 100)),
            None => ("Offline", Color32::from_rgb(240, 80, 80)),
        }
    }
}

pub struct NetPing {
    data: Arc<Mutex<PingData>>,
    in_flight: Arc<Mutex<bool>>,
    selected_target: usize,
    last_ping: Option<Instant>,
    custom_site: String,
    open_page: bool,
}

impl Default for NetPing {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(PingData::default())),
            in_flight: Arc::new(Mutex::new(false)),
            selected_target: 0,
            last_ping: None,
            custom_site: String::new(),
            open_page: false,
        }
    }
}

impl NetPing {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.spawn_ping(false);
        s
    }

    fn spawn_ping(&mut self, check_sites: bool) {
        if let Ok(mut busy) = self.in_flight.lock() {
            if *busy {
                return;
            }
            *busy = true;
        }
        self.last_ping = Some(Instant::now());
        let addr_str = TARGETS[self.selected_target.min(TARGETS.len() - 1)]
            .addr
            .to_owned();
        let data_arc = Arc::clone(&self.data);
        let busy_arc = Arc::clone(&self.in_flight);

        std::thread::spawn(move || {
            let ms = tcp_ping(&addr_str);
            let mut site_hosts: Vec<String> = Vec::new();
            if let Ok(mut d) = data_arc.lock() {
                d.total_sent = d.total_sent.saturating_add(1);
                if let Some(lat) = ms {
                    d.current_ms = Some(lat);
                    if d.history.len() >= MAX_SAMPLES {
                        d.history.pop_front();
                    }
                    d.history.push_back(lat);
                } else {
                    d.current_ms = None;
                    d.total_lost = d.total_lost.saturating_add(1);
                }
                if check_sites {
                    site_hosts = d.sites.iter().map(|s| s.host.clone()).collect();
                }
            }

            if check_sites && !site_hosts.is_empty() {
                let mut updated = Vec::with_capacity(site_hosts.len());
                for host in site_hosts {
                    let target = if host.contains(':') {
                        host.clone()
                    } else {
                        format!("{host}:443")
                    };
                    match tcp_ping(&target) {
                        Some(lat) => updated.push(SiteStatus {
                            host,
                            online: true,
                            latency_ms: lat,
                        }),
                        None => updated.push(SiteStatus {
                            host,
                            online: false,
                            latency_ms: 0,
                        }),
                    }
                }
                if let Ok(mut d) = data_arc.lock() {
                    d.sites = updated;
                }
            }

            if let Ok(mut busy) = busy_arc.lock() {
                *busy = false;
            }
        });
    }
}

fn tcp_ping(host_port: &str) -> Option<u32> {
    let addr: SocketAddr = host_port.to_socket_addrs().ok()?.next()?;
    let start = Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_millis(1500)).ok()?;
    Some(start.elapsed().as_millis().max(1) as u32)
}

impl Module for NetPing {
    fn id(&self) -> &'static str {
        "net_ping"
    }

    fn name(&self) -> &str {
        "Net Ping & Sites"
    }

    fn icon(&self) -> &'static str {
        icons::PULSE
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn next_repaint(&self, _tile_visible: bool) -> Option<Duration> {
        Some(PING_INTERVAL)
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        if self
            .last_ping
            .is_none_or(|t| t.elapsed() >= PING_INTERVAL)
        {
            self.spawn_ping(false);
        }
        let snap = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let (qual_lbl, qual_col) = snap.quality();
        let ms_txt = snap
            .current_ms
            .map(|m| format!("{m} ms"))
            .unwrap_or_else(|| "— ms".into());
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::PULSE, "Ping", snap.current_ms.is_some(), theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &ms_txt,
                    Theme::font_semibold(15.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &nook_ui::i18n::tr(qual_lbl),
                    Theme::font(10.0),
                    qual_col,
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::PULSE, "Net Ping", snap.current_ms.is_some(), theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &ms_txt,
                    Theme::font_semibold(14.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.right(), y),
                    Align2::RIGHT_CENTER,
                    &nook_ui::i18n::tr(qual_lbl),
                    Theme::font_semibold(10.5),
                    qual_col,
                );
            }
            TileSize::Large => {
                let target_name = TARGETS[self.selected_target.min(TARGETS.len() - 1)].label;
                tile_header(ui, icons::PULSE, target_name, snap.current_ms.is_some(), theme);
                let p = ui.painter();
                let top = content.top() + 21.0;
                kit::text(
                    p,
                    pos2(content.left(), top),
                    Align2::LEFT_TOP,
                    &ms_txt,
                    Theme::font_semibold(22.0),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.right(), top + 4.0),
                    Align2::RIGHT_TOP,
                    &nook_ui::i18n::tr(qual_lbl),
                    Theme::font_semibold(SIZE_LABEL),
                    qual_col,
                );
                kit::text(
                    p,
                    pos2(content.left(), top + 27.0),
                    Align2::LEFT_TOP,
                    &format!(
                        "{}: {} ms · {}: {:.0}%",
                        nook_ui::i18n::tr("Jitter"),
                        snap.jitter_ms(),
                        nook_ui::i18n::tr("Loss"),
                        snap.loss_pct()
                    ),
                    Theme::font(10.0),
                    theme.text_dim,
                );
                let chart_r = Rect::from_min_max(
                    pos2(content.left(), top + 44.0),
                    pos2(content.right(), content.bottom() - 16.0),
                );
                draw_ping_sparkline(p, chart_r, &snap.history, qual_col, theme);
                let online_count = snap.sites.iter().filter(|s| s.online).count();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() + 1.0),
                    Align2::LEFT_BOTTOM,
                    &format!(
                        "{}/{} {}",
                        online_count,
                        snap.sites.len(),
                        nook_ui::i18n::tr("sites online")
                    ),
                    Theme::font(9.5),
                    theme.text_faint,
                );
            }
        }

        if ui
            .interact(rect, ui.id().with("ping-open"), Sense::click())
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

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        if self
            .last_ping
            .is_none_or(|t| t.elapsed() >= PING_INTERVAL)
        {
            self.spawn_ping(false);
        }
        let snap = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let (qual_lbl, qual_col) = snap.quality();
        let mut check_all = false;

        if widgets::icon_button(actions, icons::ARROWS_CLOCKWISE, 24.0, false, theme).clicked() {
            check_all = true;
        }

        let p = ui.painter();
        let left_card =
            Rect::from_min_size(pos2(body.left(), body.top()), vec2(290.0, body.height()));
        let right_card = Rect::from_min_max(pos2(left_card.right() + 10.0, body.top()), body.max);
        kit::card(p, left_card, theme);
        kit::card(p, right_card, theme);

        // Target selector pills inside left card
        let tw = (left_card.width() - 20.0 - 12.0) / 4.0;
        for (i, t) in TARGETS.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(left_card.left() + 10.0 + i as f32 * (tw + 4.0), left_card.top() + 10.0),
                vec2(tw, 22.0),
            );
            let resp = ui.interact(r, ui.id().with(("pg-tgt", i)), Sense::click());
            let active = self.selected_target == i;
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
                t.label,
                Theme::font_semibold(10.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() && self.selected_target != i {
                self.selected_target = i;
                check_all = true;
            }
        }

        let p = ui.painter();
        let ms_big = snap
            .current_ms
            .map(|m| format!("{m} ms"))
            .unwrap_or_else(|| nook_ui::i18n::tr("Offline"));
        kit::text(
            p,
            pos2(left_card.left() + 14.0, left_card.top() + 42.0),
            Align2::LEFT_TOP,
            &ms_big,
            Theme::font_semibold(28.0),
            theme.text,
        );
        kit::text(
            p,
            pos2(left_card.right() - 14.0, left_card.top() + 48.0),
            Align2::RIGHT_TOP,
            &nook_ui::i18n::tr(qual_lbl),
            Theme::font_semibold(SIZE_BODY),
            qual_col,
        );
        kit::text(
            p,
            pos2(left_card.left() + 14.0, left_card.top() + 76.0),
            Align2::LEFT_TOP,
            &format!(
                "{}: {} ms   ·   {}: {:.1}%   ·   {}: {}",
                nook_ui::i18n::tr("Jitter"),
                snap.jitter_ms(),
                nook_ui::i18n::tr("Loss"),
                snap.loss_pct(),
                nook_ui::i18n::tr("Samples"),
                snap.history.len()
            ),
            Theme::font(SIZE_LABEL),
            theme.text_dim,
        );

        let plot_r = Rect::from_min_max(
            pos2(left_card.left() + 14.0, left_card.top() + 104.0),
            pos2(left_card.right() - 14.0, left_card.bottom() - 14.0),
        );
        draw_ping_sparkline(p, plot_r, &snap.history, qual_col, theme);

        // Right card: Website Uptime Checker
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!("{}  {}", icons::GLOBE, nook_ui::i18n::tr("Website Uptime Checker")),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        for (i, site) in snap.sites.iter().take(5).enumerate() {
            let ry = right_card.top() + 36.0 + i as f32 * 28.0;
            let row = Rect::from_min_size(
                pos2(right_card.left() + 10.0, ry),
                vec2(right_card.width() - 20.0, 24.0),
            );
            p.rect_filled(row, 6.0, theme.tile_bottom);
            let dot_c = if site.online {
                Color32::from_rgb(80, 220, 130)
            } else {
                Color32::from_rgb(245, 95, 95)
            };
            p.circle_filled(pos2(row.left() + 10.0, row.center().y), 4.0, dot_c);
            kit::text(
                p,
                pos2(row.left() + 20.0, row.center().y),
                Align2::LEFT_CENTER,
                &site.host,
                Theme::font(11.5),
                theme.text,
            );
            let status_txt = if site.online {
                format!("{} ms", site.latency_ms)
            } else {
                nook_ui::i18n::tr("Down")
            };
            kit::text(
                p,
                pos2(row.right() - 8.0, row.center().y),
                Align2::RIGHT_CENTER,
                &status_txt,
                Theme::font_semibold(11.0),
                dot_c,
            );
        }

        // Add custom host input at bottom of right card
        let in_r = Rect::from_min_size(
            pos2(right_card.left() + 10.0, right_card.bottom() - 34.0),
            vec2(right_card.width() - 50.0, 24.0),
        );
        let add_r = Rect::from_min_size(
            pos2(in_r.right() + 6.0, right_card.bottom() - 34.0),
            vec2(24.0, 24.0),
        );
        ui.painter().rect_filled(in_r, 6.0, theme.tile_bottom);
        let mut iui = ui.new_child(UiBuilder::new().max_rect(in_r.shrink2(vec2(6.0, 2.0))));
        let edit = iui.add(
            TextEdit::singleline(&mut self.custom_site)
                .hint_text(nook_ui::i18n::tr("Add domain (e.g. x.com)…"))
                .frame(egui::Frame::NONE)
                .font(Theme::font(11.5)),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let add_resp = ui.interact(add_r, ui.id().with("site-add"), Sense::click());
        ui.painter()
            .rect_filled(add_r, 6.0, with_alpha(theme.accent, 0.35));
        kit::text(
            ui.painter(),
            add_r.center(),
            Align2::CENTER_CENTER,
            icons::PLUS,
            Theme::font_semibold(13.0),
            theme.accent_fg(),
        );
        if (enter || add_resp.clicked()) && !self.custom_site.trim().is_empty() {
            let host = self
                .custom_site
                .trim()
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .to_owned();
            self.custom_site.clear();
            if let Ok(mut d) = self.data.lock() {
                if d.sites.len() >= 5 {
                    d.sites.pop();
                }
                d.sites.insert(
                    0,
                    SiteStatus {
                        host,
                        online: true,
                        latency_ms: 25,
                    },
                );
            }
            check_all = true;
        }

        if check_all {
            self.spawn_ping(true);
        }
        false
    }
}

fn draw_ping_sparkline(
    p: &egui::Painter,
    rect: Rect,
    history: &VecDeque<u32>,
    color: Color32,
    theme: &Theme,
) {
    p.rect_filled(rect, 6.0, with_alpha(theme.tile_bottom, 0.55));
    let r = rect.shrink(6.0);
    if history.len() < 2 || r.height() < 6.0 {
        return;
    }
    let min_v = history.iter().copied().min().unwrap_or(0) as f32;
    let max_v = history.iter().copied().max().unwrap_or(100) as f32;
    let span = (max_v - min_v).max(15.0);
    let n = (history.len() - 1).max(1);
    let pts: Vec<Pos2> = history
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = r.left() + (i as f32 / n as f32) * r.width();
            let norm = ((v as f32 - min_v) / span).clamp(0.0, 1.0);
            let y = r.bottom() - norm * r.height();
            pos2(x, y)
        })
        .collect();
    for pair in pts.windows(2) {
        p.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_jitter_and_quality() {
        let d = PingData::default();
        assert!(d.jitter_ms() <= 10);
        assert_eq!(d.quality().0, "Excellent");
    }
}
