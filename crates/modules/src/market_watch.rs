//! Currency & Crypto Market Watch: live exchange rates (`open.er-api.com`)
//! and crypto tickers (`BTC`, `ETH`, `SOL`) with an interactive currency
//! converter (inspired by NotchPixel's RatesService & TickerService).

use crate::kit;
use egui::{Align2, Color32, Rect, Sense, TextEdit, Ui, UiBuilder, pos2, vec2};
use nook_ui::bento::tile_header;
use nook_ui::module::{Module, TileCtx, TileSize};
use nook_ui::theme::{SIZE_BODY, Theme, with_alpha};
use nook_ui::{icons, widgets};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const REFRESH_RATES: Duration = Duration::from_secs(10 * 60);

const CURRENCIES: &[&str] = &["USD", "SAR", "AED", "EGP", "SDG", "EUR", "GBP", "QAR", "KWD"];

#[derive(Clone, Debug)]
struct MarketData {
    /// Rate per 1 USD.
    fiat: HashMap<String, f64>,
    btc_usd: f64,
    btc_chg: f64,
    eth_usd: f64,
    eth_chg: f64,
    sol_usd: f64,
    sol_chg: f64,
}

impl Default for MarketData {
    fn default() -> Self {
        let mut fiat = HashMap::new();
        fiat.insert("USD".into(), 1.0);
        fiat.insert("SAR".into(), 3.75);
        fiat.insert("AED".into(), 3.6725);
        fiat.insert("EGP".into(), 49.25);
        fiat.insert("SDG".into(), 601.5);
        fiat.insert("EUR".into(), 0.92);
        fiat.insert("GBP".into(), 0.79);
        fiat.insert("QAR".into(), 3.64);
        fiat.insert("KWD".into(), 0.307);
        Self {
            fiat,
            btc_usd: 89_450.0,
            btc_chg: 2.1,
            eth_usd: 2_980.0,
            eth_chg: 1.4,
            sol_usd: 184.0,
            sol_chg: 3.2,
        }
    }
}

impl MarketData {
    pub fn convert(&self, amount: f64, from: &str, to: &str) -> f64 {
        let r_from = self.fiat.get(from).copied().unwrap_or(1.0).max(1e-9);
        let r_to = self.fiat.get(to).copied().unwrap_or(1.0);
        (amount / r_from) * r_to
    }
}

pub struct MarketWatch {
    data: Arc<Mutex<MarketData>>,
    fetching: Arc<Mutex<bool>>,
    last_fetch: Option<Instant>,
    amount_str: String,
    from_idx: usize,
    to_idx: usize,
    open_page: bool,
}

impl Default for MarketWatch {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(MarketData::default())),
            fetching: Arc::new(Mutex::new(false)),
            last_fetch: None,
            amount_str: "100".into(),
            from_idx: 0, // USD
            to_idx: 1,   // SAR
            open_page: false,
        }
    }
}

impl MarketWatch {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.trigger_fetch();
        s
    }

    fn trigger_fetch(&mut self) {
        if let Ok(mut f) = self.fetching.lock() {
            if *f {
                return;
            }
            *f = true;
        }
        self.last_fetch = Some(Instant::now());
        let data_arc = Arc::clone(&self.data);
        let fetch_arc = Arc::clone(&self.fetching);

        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(5))
                .build();

            if let Ok(resp) = agent.get("https://open.er-api.com/v6/latest/USD").call()
                && let Ok(json) = resp.into_json::<serde_json::Value>()
                && let Some(rates) = json.get("rates").and_then(|r| r.as_object())
                && let Ok(mut d) = data_arc.lock()
            {
                for &code in CURRENCIES {
                    if let Some(v) = rates.get(code).and_then(|x| x.as_f64()) {
                        d.fiat.insert(code.to_owned(), v);
                    }
                }
            }

            if let Ok(resp) = agent
                .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana&vs_currencies=usd&include_24hr_change=true")
                .call()
                && let Ok(json) = resp.into_json::<serde_json::Value>()
                && let Ok(mut d) = data_arc.lock()
            {
                if let Some(b) = json.get("bitcoin") {
                    if let Some(p) = b.get("usd").and_then(|v| v.as_f64()) {
                        d.btc_usd = p;
                    }
                    if let Some(c) = b.get("usd_24h_change").and_then(|v| v.as_f64()) {
                        d.btc_chg = c;
                    }
                }
                if let Some(e) = json.get("ethereum") {
                    if let Some(p) = e.get("usd").and_then(|v| v.as_f64()) {
                        d.eth_usd = p;
                    }
                    if let Some(c) = e.get("usd_24h_change").and_then(|v| v.as_f64()) {
                        d.eth_chg = c;
                    }
                }
                if let Some(s) = json.get("solana") {
                    if let Some(p) = s.get("usd").and_then(|v| v.as_f64()) {
                        d.sol_usd = p;
                    }
                    if let Some(c) = s.get("usd_24h_change").and_then(|v| v.as_f64()) {
                        d.sol_chg = c;
                    }
                }
            }

            if let Ok(mut f) = fetch_arc.lock() {
                *f = false;
            }
        });
    }
}

impl Module for MarketWatch {
    fn id(&self) -> &'static str {
        "market_watch"
    }

    fn name(&self) -> &str {
        "Currency & Crypto"
    }

    fn icon(&self) -> &'static str {
        icons::CURRENCY_DOLLAR
    }

    fn size_hint(&self) -> TileSize {
        TileSize::Small
    }

    fn ui_tile(&mut self, ui: &mut Ui, ctx: &mut TileCtx<'_>) {
        if self
            .last_fetch
            .is_none_or(|t| t.elapsed() >= REFRESH_RATES)
        {
            self.trigger_fetch();
        }
        let snap = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let content = ui.max_rect();
        let rect = content.expand(8.0);
        let size = kit::tile_size(content);
        let theme = ctx.theme;

        match size {
            TileSize::Small => {
                tile_header(ui, icons::CURRENCY_DOLLAR, "Market", false, theme);
                let p = ui.painter();
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() - 14.0),
                    Align2::LEFT_BOTTOM,
                    &format!("${:.0}", snap.btc_usd),
                    Theme::font_semibold(13.5),
                    theme.text,
                );
                kit::text(
                    p,
                    pos2(content.left(), content.bottom()),
                    Align2::LEFT_BOTTOM,
                    &format!("BTC ({:+.1}%)", snap.btc_chg),
                    Theme::font(9.5),
                    if snap.btc_chg >= 0.0 {
                        Color32::from_rgb(80, 220, 130)
                    } else {
                        Color32::from_rgb(245, 100, 100)
                    },
                );
            }
            TileSize::Wide => {
                tile_header(ui, icons::CURRENCY_DOLLAR, "Currency & Crypto", false, theme);
                let p = ui.painter();
                let y = content.top() + 29.0;
                kit::text(
                    p,
                    pos2(content.left(), y),
                    Align2::LEFT_CENTER,
                    &format!("BTC ${:.0}", snap.btc_usd),
                    Theme::font_semibold(12.5),
                    theme.text,
                );
                let chg_col = if snap.btc_chg >= 0.0 {
                    Color32::from_rgb(80, 220, 130)
                } else {
                    Color32::from_rgb(245, 100, 100)
                };
                kit::text(
                    p,
                    pos2(content.right(), y),
                    Align2::RIGHT_CENTER,
                    &format!("{:+.1}%", snap.btc_chg),
                    Theme::font_semibold(10.5),
                    chg_col,
                );
            }
            TileSize::Large => {
                tile_header(ui, icons::CURRENCY_DOLLAR, "Currency & Crypto", false, theme);
                let p = ui.painter();
                let top = content.top() + 22.0;
                for (i, (sym, price, chg)) in [
                    ("BTC", snap.btc_usd, snap.btc_chg),
                    ("ETH", snap.eth_usd, snap.eth_chg),
                    ("SOL", snap.sol_usd, snap.sol_chg),
                ]
                .iter()
                .enumerate()
                {
                    let ry = top + i as f32 * 20.0;
                    kit::text(
                        p,
                        pos2(content.left(), ry),
                        Align2::LEFT_TOP,
                        &format!("{sym}  ${price:.0}"),
                        Theme::font_semibold(11.5),
                        theme.text,
                    );
                    kit::text(
                        p,
                        pos2(content.right(), ry),
                        Align2::RIGHT_TOP,
                        &format!("{chg:+.1}%"),
                        Theme::font_semibold(10.5),
                        if *chg >= 0.0 {
                            Color32::from_rgb(80, 220, 130)
                        } else {
                            Color32::from_rgb(245, 100, 100)
                        },
                    );
                }
                let sar = snap.convert(1.0, "USD", "SAR");
                let eur = snap.convert(1.0, "USD", "EUR");
                kit::text(
                    p,
                    pos2(content.left(), content.bottom() + 1.0),
                    Align2::LEFT_BOTTOM,
                    &format!("USD: {sar:.2} SAR · {eur:.2} EUR"),
                    Theme::font(10.0),
                    theme.text_dim,
                );
            }
        }

        if ui
            .interact(rect, ui.id().with("mw-open"), Sense::click())
            .clicked()
        {
            self.open_page = true;
        }
    }

    fn take_page_request(&mut self) -> bool {
        std::mem::take(&mut self.open_page)
    }

    fn page_size(&self) -> Option<egui::Vec2> {
        Some(vec2(568.0, 260.0))
    }

    fn ui_page(&mut self, ui: &mut Ui, actions: &mut Ui, body: Rect, theme: &Theme) -> bool {
        let snap = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let mut refresh = false;

        if widgets::icon_button(actions, icons::ARROWS_CLOCKWISE, 24.0, false, theme).clicked() {
            refresh = true;
        }

        let p = ui.painter();
        let conv_card =
            Rect::from_min_size(pos2(body.left(), body.top()), vec2(285.0, body.height()));
        let right_card = Rect::from_min_max(pos2(conv_card.right() + 10.0, body.top()), body.max);
        kit::card(p, conv_card, theme);
        kit::card(p, right_card, theme);

        // Left: Interactive Currency Converter
        kit::text(
            p,
            pos2(conv_card.left() + 12.0, conv_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!(
                "{}  {}",
                icons::ARROWS_LEFT_RIGHT,
                nook_ui::i18n::tr("Currency Converter")
            ),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        let in_r = Rect::from_min_size(
            pos2(conv_card.left() + 12.0, conv_card.top() + 36.0),
            vec2(conv_card.width() - 24.0, 28.0),
        );
        ui.painter().rect_filled(in_r, 6.0, theme.tile_bottom);
        let mut iui = ui.new_child(UiBuilder::new().max_rect(in_r.shrink2(vec2(8.0, 4.0))));
        iui.add(
            TextEdit::singleline(&mut self.amount_str)
                .hint_text("100")
                .frame(egui::Frame::NONE)
                .font(Theme::font_semibold(SIZE_BODY)),
        );

        // From currency row
        let p = ui.painter();
        kit::text(
            p,
            pos2(conv_card.left() + 12.0, conv_card.top() + 72.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("From"),
            Theme::font(10.5),
            theme.text_dim,
        );
        let cw = (conv_card.width() - 24.0 - 32.0) / 9.0;
        for (i, &code) in CURRENCIES.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(conv_card.left() + 12.0 + i as f32 * (cw + 4.0), conv_card.top() + 88.0),
                vec2(cw, 20.0),
            );
            let resp = ui.interact(r, ui.id().with(("mw-from", i)), Sense::click());
            let active = self.from_idx == i;
            let p = ui.painter();
            p.rect_filled(
                r,
                5.0,
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
                code,
                Theme::font_semibold(9.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() {
                self.from_idx = i;
            }
        }

        // To currency row
        let p = ui.painter();
        kit::text(
            p,
            pos2(conv_card.left() + 12.0, conv_card.top() + 114.0),
            Align2::LEFT_TOP,
            &nook_ui::i18n::tr("To"),
            Theme::font(10.5),
            theme.text_dim,
        );
        for (i, &code) in CURRENCIES.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(conv_card.left() + 12.0 + i as f32 * (cw + 4.0), conv_card.top() + 130.0),
                vec2(cw, 20.0),
            );
            let resp = ui.interact(r, ui.id().with(("mw-to", i)), Sense::click());
            let active = self.to_idx == i;
            let p = ui.painter();
            p.rect_filled(
                r,
                5.0,
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
                code,
                Theme::font_semibold(9.0),
                if active { theme.accent_fg() } else { theme.text_dim },
            );
            if resp.clicked() {
                self.to_idx = i;
            }
        }

        let p = ui.painter();
        let amount = self.amount_str.trim().parse::<f64>().unwrap_or(1.0);
        let from_c = CURRENCIES[self.from_idx.min(CURRENCIES.len() - 1)];
        let to_c = CURRENCIES[self.to_idx.min(CURRENCIES.len() - 1)];
        let result = snap.convert(amount, from_c, to_c);
        let res_box = Rect::from_min_max(
            pos2(conv_card.left() + 12.0, conv_card.bottom() - 46.0),
            pos2(conv_card.right() - 12.0, conv_card.bottom() - 10.0),
        );
        p.rect_filled(res_box, 8.0, with_alpha(theme.accent, 0.22));
        kit::text(
            p,
            res_box.center(),
            Align2::CENTER_CENTER,
            &format!("{amount:.2} {from_c}  =  {result:.2} {to_c}"),
            Theme::font_semibold(14.5),
            theme.text,
        );

        // Right card: Live Crypto & Popular Rates
        kit::text(
            p,
            pos2(right_card.left() + 12.0, right_card.top() + 10.0),
            Align2::LEFT_TOP,
            &format!(
                "{}  {}",
                icons::TREND_UP,
                nook_ui::i18n::tr("Crypto & Exchange Rates")
            ),
            Theme::font_semibold(SIZE_BODY),
            theme.text,
        );

        let rows = [
            ("Bitcoin (BTC)", format!("${:.0}", snap.btc_usd), snap.btc_chg),
            ("Ethereum (ETH)", format!("${:.0}", snap.eth_usd), snap.eth_chg),
            ("Solana (SOL)", format!("${:.2}", snap.sol_usd), snap.sol_chg),
        ];
        for (i, (name, price, chg)) in rows.iter().enumerate() {
            let ry = right_card.top() + 36.0 + i as f32 * 28.0;
            let r = Rect::from_min_size(
                pos2(right_card.left() + 10.0, ry),
                vec2(right_card.width() - 20.0, 24.0),
            );
            p.rect_filled(r, 6.0, theme.tile_bottom);
            kit::text(
                p,
                pos2(r.left() + 8.0, r.center().y),
                Align2::LEFT_CENTER,
                name,
                Theme::font(11.5),
                theme.text,
            );
            let col = if *chg >= 0.0 {
                Color32::from_rgb(80, 220, 130)
            } else {
                Color32::from_rgb(245, 100, 100)
            };
            kit::text(
                p,
                pos2(r.right() - 8.0, r.center().y),
                Align2::RIGHT_CENTER,
                &format!("{price} ({chg:+.1}%)"),
                Theme::font_semibold(11.0),
                col,
            );
        }

        // 4 Fiat quick rates at bottom of right card
        for (i, &code) in ["SAR", "AED", "EGP", "EUR"].iter().enumerate() {
            let c = i % 2;
            let rw = i / 2;
            let w = (right_card.width() - 26.0) * 0.5;
            let r = Rect::from_min_size(
                pos2(
                    right_card.left() + 10.0 + c as f32 * (w + 6.0),
                    right_card.top() + 128.0 + rw as f32 * 34.0,
                ),
                vec2(w, 28.0),
            );
            p.rect_filled(r, 6.0, theme.tile_bottom);
            let val = snap.convert(1.0, "USD", code);
            kit::text(
                p,
                pos2(r.left() + 8.0, r.center().y),
                Align2::LEFT_CENTER,
                &format!("USD/{code}"),
                Theme::font(11.0),
                theme.text_dim,
            );
            kit::text(
                p,
                pos2(r.right() - 8.0, r.center().y),
                Align2::RIGHT_CENTER,
                &format!("{val:.2}"),
                Theme::font_semibold(11.5),
                theme.accent_fg(),
            );
        }

        if refresh {
            self.trigger_fetch();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_conversion_math() {
        let d = MarketData::default();
        let sar = d.convert(100.0, "USD", "SAR");
        assert!((sar - 375.0).abs() < 0.01);
    }
}
