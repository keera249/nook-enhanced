//! Nook's utility modules. Each lives in its own folder and implements
//! [`nook_ui::module::Module`]; [`register_all`] wires them into a registry in
//! the default layout order, followed by any [`plugins`] found on disk.

pub mod ambient_sound;
pub mod calculator;
pub mod calendar;
pub mod clipboard;
pub mod coffee;
pub mod color_picker;
pub mod control_center;
pub mod draw;
pub mod file_shelf;
pub mod font_detector;
pub mod habit_water;
pub mod kit;
pub mod lens;
pub mod market_watch;
pub mod media;
pub mod net_ping;
pub mod notes;
pub mod phone_link;
pub mod plugins;
pub mod qr_code;
pub mod quick_launch;
pub mod reminders;
pub mod screen_time;
pub mod screenshot;
mod shot_editor;
mod snipper;
pub mod stub;
pub mod system_monitor;
pub mod text_extractor;
pub mod timer;
pub mod todo_focus;
pub mod translator;
pub mod unit_converter;
pub mod voice_memo;
pub mod wallpaper;
pub mod weather;
pub mod world_clock;

use nook_ui::module::Registry;

/// Default tile order. Packs 3 full 8x3 pages (24 cells each = 72 cells total)
/// with aligned column pairs and zero page overflow.
pub const DEFAULT_ORDER: &[&str] = &[
    // Page 1 (24 cells)
    "timer",
    "notes",
    "media",
    "coffee",
    "screenshot",
    "clipboard",
    "lens",
    "plugin:prayer-times",
    "plugin:game-launcher",
    "plugin:dawwar",
    "plugin:volume-mixer",
    "plugin:power-mode",
    // Page 2 (24 cells)
    "file_shelf",
    "calendar",
    "system_monitor",
    "quick_launch_2",
    "quick_launch",
    "calculator",
    "ocr",
    "color_picker",
    "draw",
    "phone_link",
    "font_detector",
    "wallpaper",
    "qr_code",
    "voice_memo",
    "plugins",
    "settings",
    // Page 3 (24 cells — 12 NotchPixel Wide 2x1 modules = 4 columns x 3 rows)
    "control_center",
    "weather",
    "todo_focus",
    "net_ping",
    "world_clock",
    "market_watch",
    "screen_time",
    "reminders",
    "unit_converter",
    "translator",
    "habit_water",
    "ambient_sound",
];

pub fn register_all(reg: &mut Registry) {
    reg.register(Box::<timer::Timer>::default());
    reg.register(Box::<coffee::Coffee>::default());
    reg.register(Box::<media::Media>::default());
    reg.register(Box::<notes::Notes>::default());
    reg.register(Box::<clipboard::Clipboard>::default());
    reg.register(Box::<color_picker::ColorPicker>::default());
    reg.register(Box::<screenshot::Screenshot>::default());
    reg.register(Box::<text_extractor::TextExtractor>::default());
    reg.register(Box::<lens::Lens>::default());
    reg.register(Box::<draw::Draw>::default());
    reg.register(Box::<phone_link::PhoneLink>::default());
    reg.register(Box::<file_shelf::FileShelf>::default());
    // Past the first grid: the second page.
    reg.register(Box::<font_detector::FontDetector>::default());
    reg.register(Box::<wallpaper::Wallpaper>::default());
    reg.register(Box::<calculator::Calculator>::default());
    reg.register(Box::<system_monitor::SystemMonitor>::default());
    reg.register(Box::<calendar::Calendar>::default());
    reg.register(Box::<quick_launch::QuickLaunch>::default());
    reg.register(Box::new(quick_launch::QuickLaunch::second()));
    reg.register(Box::<qr_code::QrCodeModule>::default());
    reg.register(Box::<voice_memo::VoiceMemo>::default());
    // The plugin manager, then third-party tiles from the plugin folders.
    reg.register(Box::<plugins::PluginsTile>::default());
    // NotchPixel-inspired utility suite (12 modules).
    reg.register(Box::new(control_center::ControlCenter::new()));
    reg.register(Box::new(weather::Weather::new()));
    reg.register(Box::new(todo_focus::TodoFocus::new()));
    reg.register(Box::new(net_ping::NetPing::new()));
    reg.register(Box::new(world_clock::WorldClock::new()));
    reg.register(Box::new(market_watch::MarketWatch::new()));
    reg.register(Box::new(screen_time::ScreenTime::new()));
    reg.register(Box::new(reminders::Reminders::new()));
    reg.register(Box::new(unit_converter::UnitConverter::new()));
    reg.register(Box::<translator::Translator>::default());
    reg.register(Box::<habit_water::HabitWater>::default());
    reg.register(Box::<ambient_sound::AmbientSound>::default());
    plugins::register(reg);
}
