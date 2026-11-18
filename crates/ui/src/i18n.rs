//! Internationalization (i18n) and localization for Nook.
//!
//! Provides translation and Arabic text shaping across the entire application.

use std::sync::atomic::{AtomicU8, Ordering};
use serde::{Deserialize, Serialize};
use crate::arabic::shape_bidi;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    #[serde(rename = "ar", alias = "arabic")]
    Arabic = 0,
    #[serde(rename = "en", alias = "english")]
    English = 1,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Arabic, Language::English];

    pub fn label(self) -> &'static str {
        match self {
            Language::Arabic => "العربية",
            Language::English => "English",
        }
    }
}

static CURRENT_LANG: AtomicU8 = AtomicU8::new(Language::Arabic as u8);

/// Sets the active global language.
pub fn set_language(lang: Language) {
    CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
}

/// Returns the currently active language.
pub fn current_language() -> Language {
    match CURRENT_LANG.load(Ordering::Relaxed) {
        0 => Language::Arabic,
        _ => Language::English,
    }
}

/// Returns true if Arabic is currently active.
pub fn is_arabic() -> bool {
    current_language() == Language::Arabic
}

/// Translates a key string into the active language, applying Arabic shaping if needed.
pub fn tr(key: &str) -> String {
    if !is_arabic() {
        return shape_bidi(key);
    }
    let ar = translate_ar(key);
    if ar != key {
        return shape_bidi(ar);
    }
    if let Some(dyn_ar) = translate_ar_dynamic(key) {
        return shape_bidi(&dyn_ar);
    }
    shape_bidi(ar)
}

fn translate_prayer_name_ar(name: &str) -> &str {
    match name.trim() {
        "Fajr" => "الفجر",
        "Sunrise" => "الشروق",
        "Dhuhr" => "الظهر",
        "Asr" => "العصر",
        "Maghrib" => "المغرب",
        "Isha" => "العشاء",
        other => translate_ar(other),
    }
}

/// Handles dynamic/parameterized strings (e.g. from plugins or formatted status labels).
fn translate_ar_dynamic(key: &str) -> Option<String> {
    if let Some(rest) = key.strip_prefix("Prayer Times - Next: ") {
        if let Some((prayer, time)) = rest.split_once(" at ") {
            let p_ar = translate_prayer_name_ar(prayer);
            return Some(format!("مواقيت الصلاة - التالي: {p_ar} عند {time}"));
        }
    }
    if let Some(rest) = key.strip_prefix("Next: ") {
        let p_ar = translate_prayer_name_ar(rest);
        if p_ar != rest {
            return Some(format!("التالي: {p_ar}"));
        }
    }
    if let Some(rest) = key.strip_prefix("Azan ") {
        let p_ar = translate_prayer_name_ar(rest);
        return Some(format!("أذان {p_ar}"));
    }
    for (en, ar) in [
        ("Fajr ", "الفجر "),
        ("Sunrise ", "الشروق "),
        ("Dhuhr ", "الظهر "),
        ("Asr ", "العصر "),
        ("Maghrib ", "المغرب "),
        ("Isha ", "العشاء "),
    ] {
        if let Some(rest) = key.strip_prefix(en) {
            let rest_ar = rest.replace("h", "س ").replace("m", "د");
            return Some(format!("{ar}{}", rest_ar.trim()));
        }
    }
    if let Some((countdown, city)) = key.split_once(" left - ") {
        let city_ar = translate_ar(city.trim());
        return Some(format!("متبقي {countdown} - {city_ar}"));
    }
    if let Some(rest) = key.strip_prefix("Remaining: ") {
        return Some(format!("المتبقي: {rest}"));
    }
    if let Some(rest) = key.strip_prefix("Recite & Count: ") {
        return Some(format!("سبّح وعدّ: {rest}"));
    }
    if let Some(rest) = key.strip_prefix("Virtue: ") {
        return Some(format!("الفضل: {rest}"));
    }
    if let Some(rest) = key.strip_prefix("Current: ") {
        return Some(format!("الموقع الحالي: {rest}"));
    }
    if let Some(rest) = key.strip_prefix("Mixer (") {
        if let Some(num) = rest.strip_suffix(')') {
            return Some(format!("الموازن ({num})"));
        }
    }
    if let Some(rest) = key.strip_prefix("Volume Mixer (") {
        if let Some(num) = rest.strip_suffix(" apps)") {
            return Some(format!("موازن الصوت ({num} تطبيقات)"));
        }
    }
    if let Some(app) = key.strip_suffix(" (Playing)") {
        return Some(format!("{app} (يعمل الآن)"));
    }
    if let Some(app) = key.strip_prefix("Mute / Unmute ") {
        return Some(format!("كتم / إلغاء كتم {app}"));
    }
    if let Some(mode) = key.strip_prefix("Switched Power Mode to ") {
        return Some(format!("تم تغيير وضع الطاقة إلى {}", translate_ar(mode)));
    }
    if let Some(plan) = key.strip_prefix("Activated plan: ") {
        return Some(format!("تم تفعيل الخطة: {}", translate_ar(plan)));
    }
    if let Some(rest) = key.strip_prefix("Power: ") {
        if let Some(mode) = rest.strip_suffix(" (click to toggle Gaming Performance)") {
            return Some(format!(
                "الطاقة: {} (انقر لتبديل الأداء العالي)",
                translate_ar(mode)
            ));
        }
    }
    if let Some(rest) = key.strip_prefix("Dropped: ") {
        return Some(format!("تم إفلات: {rest}"));
    }
    if let Some(rest) = key.strip_prefix("Saved ") {
        return Some(format!("تم الحفظ: {rest}"));
    }
    None
}

/// Translates a key string into Arabic raw form.
pub fn translate_ar(key: &str) -> &str {
    match key {
        // Module Names & Titles (including case variants)
        "Timer" => "المؤقت",
        "Coffee" => "مانع السكون",
        "Media" => "مشغل الوسائط",
        "Notes" => "الملاحظات",
        "Clipboard" => "الحافظة",
        "Screenshot" => "لقطة الشاشة",
        "Screen Capture" => "لقطة الشاشة",
        "Lens" | "Lens search" => "عدسة البحث",
        "Calendar" => "التقويم",
        "OCR" | "Text Extractor" | "Text extractor" => "قارئ النصوص",
        "Color Picker" | "Color picker" => "منتقي الألوان",
        "Draw" | "Draw on screen" => "لوحة الرسم",
        "Phone Link" | "Phone link" => "ربط الهاتف",
        "File Shelf" | "File shelf" => "رف الملفات",
        "Font Detector" | "Font detector" => "كاشف الخطوط",
        "Wallpaper" => "الخلفيات",
        "Calculator" => "الآلة الحاسبة",
        "System Monitor" | "System monitor" | "System" => "مراقب النظام",
        "Quick Launch" | "Quick launch" => "تشغيل سريع",
        "Quick Launch 2" | "Quick launch 2" => "تشغيل سريع ٢",
        "QR Code" | "QR code" => "رمز الاستجابة",
        "Voice Memo" | "Voice memo" => "مذكرات صوتية",
        "Plugins" => "الإضافات",
        "Prayer Times" => "مواقيت الصلاة",
        "Game Launcher" => "مشغل الألعاب",
        "App Volume Mixer" | "Volume Mixer" => "موازن الصوت",
        "Power & Performance" => "الطاقة والأداء",
        "Dawwar" => "دوّار",
        "Master Volume" => "الصوت الرئيسي",
        "Muted" => "مكتوم",
        "Mute / Unmute Master" => "كتم / إلغاء كتم الصوت",
        "Toggle Master Mute" => "كتم الصوت الرئيسي",
        "Refresh open audio apps" => "تحديث تطبيقات الصوت المفتوحة",
        "Power Mode" => "وضع الطاقة",
        "Power Saver" | "Power saver" => "موفر الطاقة",
        "Balanced" => "متوازن",
        "Best Performance" => "أفضل أداء",
        "Ultimate Performance" => "الأداء الأقصى",
        "High performance" | "High Performance" => "أداء عالي",
        "Windows Performance Mode" => "وضع أداء ويندوز",
        "Power Plans (powercfg)" => "خطط الطاقة (powercfg)",
        "Unlock Ultimate Plan" => "تفعيل خطة الأداء الأقصى",
        "Unlock hidden High & Ultimate Performance plans" => "تفعيل خطط الأداء العالي والأقصى المخفية",
        "Unlocked performance power plans!" => "تم تفعيل خطط الأداء العالي!",
        "All power plans are already unlocked" => "جميع خطط الطاقة مفعلة بالفعل",
        "Toggle Gaming Best Performance" => "تبديل وضع الأداء الأقصى للألعاب",
        "Refresh power state" => "تحديث حالة الطاقة",
        "Open Windows Power Options" => "فتح خيارات الطاقة في ويندوز",
        "Turbo ON" => "التيربو مفعل",
        "Boost Now" => "تعزيز الأداء",
        "Active" => "نشط",
        "Auto with Nook" => "تلقائي مع نووك",
        "Dawwar App" => "تطبيق دوّار",
        "Appearance & Wheel" => "المظهر والعجلة",
        "AI & Speech" => "الذكاء الاصطناعي والصوت",
        "Quick Convert & Guide" => "تحويل سريع ودليل",
        "Shift | Ctrl | Space | Alt" => "Shift | Ctrl | Space | Alt (عجلة دوّار)",
        "File Converter and Radial AI Wheel" => "محول الملفات وعجلة الذكاء الاصطناعي",
        "INSTALLED" => "المثبتة",
        "Install" => "تثبيت",
        "Cancel" => "إلغاء",
        "Read code" => "عرض الكود",
        "Open the plugins folder" => "فتح مجلد الإضافات",
        "Drop a plugin .zip or .7z to install" => "أفلت ملف إضافة .zip أو .7z لتثبيتها",
        "One plugin or a bundle of several" => "إضافة واحدة أو حزمة إضافات",
        "No plugins yet. Install one from the marketplace, or drop a .zip here." => "لا توجد إضافات بعد. يمكنك سحب ملف .zip هنا لتثبيتها.",
        "Installed. Its tile is in the grid now, at the end." => "تم التثبيت بنجاح. تمت إضافتها إلى الشبكة.",
        "Downloading and checking…" => "جاري التحميل والفحص…",
        "Show on the grid" => "معروض في الشبكة",
        "Hidden from the grid" => "مخفي من الشبكة",
        "Restart" => "إعادة تشغيل",
        "Sure?" => "متأكد؟",
        "Update" => "تحديث",
        "Up to date" => "محدّث",
        "Updated" => "تم التحديث",
        "Stopped · click to restart" => "متوقف · انقر لإعادة التشغيل",
        "Islamic prayer times & Azan countdown timer with dynamic notch ring and auto location detection." => "مواقيت الصلاة والعد التنازلي للأذان مع حلقة ديناميكية في النوتش وتحديد تلقائي للموقع والأذكار.",
        "Finds your installed Steam, Epic and GOG games by itself, or drop any game's .exe, shortcut or folder. One click to play." => "يكتشف ألعاب Steam و Epic و GOG تلقائياً، أو اسحب أي لعبة لتشغيلها بنقرة واحدة.",
        "Per-app volume mixer: control or mute the volume of each open game and app individually." => "موازن صوت لكل تطبيق: تحكم بمستوى صوت كل لعبة أو تطبيق مفتوح بشكل مستقل.",
        "Switch between Gaming Best Performance, Ultimate Performance, Balanced, and Power Saver modes in one click." => "التبديل السريع بين الأداء الأقصى للألعاب، المتوازن، وموفر الطاقة بنقرة واحدة.",
        "Dawwar radial file converter & AI wheel in the background with full in-notch settings and quick file drop conversion." => "محول الملفات الدائري وعجلة الذكاء الاصطناعي في الخلفية مع تحويل فوري بالسحب والإفلات.",
        "Settings" => "الإعدادات",

        // Settings - Sections
        "Appearance" => "المظهر",
        "Behavior" => "السلوك",
        "Customize" => "تخصيص متقدم",
        "Advanced" => "خيارات متقدمة",

        // Settings - Appearance Controls & Tooltips
        "Theme" => "السمة",
        "Auto" => "تلقائي",
        "Dark" => "داكن",
        "Light" => "فاتح",
        "Accent" => "لون التمييز",
        "Windows" => "ويندوز",
        "Custom" => "مخصص",
        "Position" => "الموقع",
        "Left" => "يسار",
        "Center" => "وسط",
        "Right" => "يمين",
        "Shape" => "الشكل",
        "Island" => "جزيرة ديناميكية",
        "Notch" => "نوتش شاشة",
        "Notch width" => "عرض النوتش",
        "Mini in full window" => "تصغير عند التكبير",
        "Hide in full window" => "إخفاء عند ملء الشاشة",
        "Slimmer bar while the active app fills the screen" => "شريط أنحف عندما يملأ التطبيق النشط الشاشة",
        "Tuck away completely while a fullscreen game or video is up" => "الاختفاء تماماً أثناء تشغيل لعبة أو فيديو بملء الشاشة",
        "Move to whichever screen the mouse is on" => "الانتقال تلقائياً إلى الشاشة التي يوجد بها المؤشر",
        "Look" => "المظهر",
        "Pick a color" => "اختر لوناً",
        "Choose color" => "اختيار اللون",
        "Match Windows accent" => "مطابقة لون تمييز ويندوز",
        "Back to the default dark glass" => "العودة إلى المظهر الزجاجي الافتراضي",
        "Switch to DirectX if OpenGL misbehaves on your GPU" => "التبديل إلى DirectX إذا واجهت مشاكل مع OpenGL",
        "Opens Windows Settings > Personalization > Colors" => "يفتح إعدادات ويندوز > إضفاء طابع شخصي > الألوان",

        // Settings - Behavior Controls
        "Language" => "اللغة",
        "Monitor" => "الشاشة",
        "Primary" => "الرئيسية",
        "Follow cursor" => "تتبع الماوس",
        "Hover delay" => "سرعة الاستجابة",
        "Fast" => "سريع",
        "Normal" => "عادي",
        "Calm" => "هادئ",
        "Always on top" => "دائماً في المقدمة",
        "Launch at startup" => "تشغيل مع بدء ويندوز",

        // Settings - Customize / Advanced
        "Material" => "نوع الزجاج",
        "Glass" => "زجاج عادي",
        "Liquid Glass" => "زجاج سائل",
        "Solid" => "معتم",
        "Background" => "لون الخلفية",
        "Glass opacity" => "شفافية الزجاج",
        "Accent color" => "لون التمييز",
        "Grain" => "حبيبات الزجاج",
        "Grain size" => "حجم الحبيبات",
        "Fine" => "ناعم",
        "Medium" => "متوسط",
        "Coarse" => "خشن",
        "Off" => "معطل",
        "Reset style" => "استعادة المظهر",
        "Manage plugins" => "إدارة الإضافات",
        "Plugin dev mode" => "وضع تطوير الإضافات",
        "Renderer" => "محرك الرسوميات",
        "Open log" => "فتح السجل",
        "Windows transparency is off" => "تأثيرات الشفافية معطلة في ويندوز",
        "Turn on" => "تفعيل",
        "Nook Enhanced" => "نووك المطور",
        "tinted" => "ملون",
        "neutral" => "محايد",
        "no grain" => "بدون حبيبات",
        "grain" => "حبيبات",
        "on" => "مفعل",
        "off" => "معطل",
        "Dev mode" => "وضع المطور",
        "Restarts a plugin when you save its files" => "إعادة تشغيل الإضافة عند حفظ ملفاتها",
        "Shows nook.log in Explorer, to attach to a bug report" => "عرض ملف nook.log في المستكشف",

        // Bento Grid & Shell Actions
        "Click outside to fold" => "انقر بالخارج للطي",
        "Coming soon" => "قريباً",
        "Soon" => "قريباً",
        "Edit modules" => "ترتيب الوحدات",
        "Done" => "تم",
        "Quit Nook" => "إنهاء البرنامج",
        "Quit" => "إنهاء",
        "Exit" => "خروج",
        "Pin" => "تثبيت",
        "Unpin" => "إلغاء التثبيت",
        "Close" => "إغلاق",
        "Back" => "رجوع",
        "Always shown" => "معروض دائماً",
        "Shown. Click to hide" => "معروض. انقر للإخفاء",
        "Hidden. Click to show" => "مخفي. انقر للعرض",
        "Small" => "صغير",
        "Wide" => "عريض",
        "Large" => "كبير",

        // Timer
        "Stopwatch" => "ساعة الإيقاف",
        "Countdown" => "العد التنازلي",
        "Start" => "بدء",
        "Pause" => "إيقاف مؤقت",
        "Resume" => "استئناف",
        "Reset" => "إعادة ضبط",
        "Dismiss" => "إغلاق",
        "One more minute" => "دقيقة إضافية",
        "1 minute more" => "دقيقة إضافية",
        "1 minute less" => "إنقاص دقيقة",
        "Double-click to type a time: 5, 5:30, 90s or 1h30m" => "انقر مرتين لإدخال الوقت: 5 أو 5:30 أو 90s أو 1h30m",

        // Coffee
        "Always" => "دائماً",
        "Awake" => "نشط",
        "Click to change how long" => "انقر لتغيير المدة",
        "15 min" => "١٥ دقيقة",
        "30 min" => "٣٠ دقيقة",
        "45 min" => "٤٥ دقيقة",
        "60 min" => "٦٠ دقيقة",
        "10 min" => "١٠ دقائق",
        "20 min" => "٢٠ دقيقة",
        "1 hour" | "1 Hour" => "ساعة واحدة",
        "2 hours" | "2 Hours" => "ساعتان",
        "3 Hours" => "٣ ساعات",
        "4 hours" => "٤ ساعات",
        "30 min (Default)" => "٣٠ دقيقة (افتراضي)",

        // Notes
        "New note" => "ملاحظة جديدة",
        "Start typing. The first line is the title." => "ابدأ بالكتابة. السطر الأول هو العنوان.",
        "Write something" => "اكتب شيئاً...",
        "No notes. Press + to start one." => "لا توجد ملاحظات. اضغط + للبدء.",
        "Editing" => "تعديل",
        "Move this note to the Recycle Bin" => "نقل الملاحظة إلى سلة المحذوفات",
        "Just now" => "الآن",

        // Clipboard
        "Copied" => "تم النسخ",
        "Copy something" => "انسخ نصاً أو صورة",
        "Show all" => "عرض الكل",
        "Clear history" => "مسح السجل",
        "Nothing copied yet. Text and images you copy show up here." => "لا يوجد شيء منسوخ بعد. ما تنسخه سيظهر هنا.",
        "Image" => "صورة",
        "Remove" => "إزالة",
        "chars" => "حرف",
        "now" => "الآن",

        // Calendar
        "Today" => "اليوم",
        "Tomorrow" => "غداً",
        "Yesterday" => "أمس",
        "Next month" => "الشهر القادم",
        "Previous month" => "الشهر السابق",
        "All day" => "طوال اليوم",
        "Nothing planned" => "لا توجد مواعيد",
        "COMING UP" => "المواعيد القادمة",
        "No more events today" => "لا توجد أحداث أخرى اليوم",
        "Next: " => "التالي: ",
        "Now: " => "الآن: ",
        "Delete" => "حذف",
        "Add: 14:30 Dentist" => "إضافة: 14:30 موعد الطبيب",
        "January" | "Jan" => "يناير",
        "February" | "Feb" => "فبراير",
        "March" | "Mar" => "مارس",
        "April" | "Apr" => "أبريل",
        "May" => "مايو",
        "June" | "Jun" => "يونيو",
        "July" | "Jul" => "يوليو",
        "August" | "Aug" => "أغسطس",
        "September" | "Sep" => "سبتمبر",
        "October" | "Oct" => "أكتوبر",
        "November" | "Nov" => "نوفمبر",
        "December" | "Dec" => "ديسمبر",
        "Monday" => "الإثنين",
        "Tuesday" => "الثلاثاء",
        "Wednesday" => "الأربعاء",
        "Thursday" => "الخميس",
        "Friday" => "الجمعة",
        "Saturday" => "السبت",
        "Sunday" => "الأحد",
        "Mon" => "إث",
        "Tue" => "ثل",
        "Wed" => "أر",
        "Thu" => "خم",
        "Fri" => "جم",
        "Sat" => "سب",
        "Sun" => "أح",

        // Calculator
        "25% of 80  \u{00B7}  120 km in mi  \u{00B7}  0xff in hex  \u{00B7}  sin(45 deg)" => "مثال: 25% of 80  ·  120 km in mi  ·  sin(45 deg)",
        "Click to copy  \u{00B7}  Right-click for more formats" => "انقر للنسخ  ·  زر أيمن لصيغ إضافية",
        "HISTORY" => "السجل",
        "Your calculations will show up here" => "ستظهر عملياتك الحسابية هنا",
        "Expression" => "المعادلة",
        "Clear the expression (Esc)" => "مسح المعادلة (Esc)",
        "Evaluate and add to history (Enter)" => "حساب وإضافة للسجل (Enter)",
        "Division by zero" => "لا يمكن القسمة على صفر",
        "Unmatched '('" => "قوس '(' غير مغلق",
        "Unexpected ')'" => "قوس ')' غير متوقع",
        "Could not understand that" => "تعذر فهم المعادلة",
        "Incomplete expression" => "معادلة غير مكتملة",
        "Missing number" => "رقم مفقود",
        "Unknown unit" => "وحدة غير معروفة",
        "Incompatible units" => "وحدات غير متوافقة",

        // Color Picker
        "Click to copy" => "انقر للنسخ",
        "Pick a color from the screen (Ctrl+Alt+C)" => "التقاط لون من الشاشة (Ctrl+Alt+C)",
        "Clear" => "مسح",
        "No colors picked yet." => "لم يتم التقاط أي ألوان بعد.",
        "Click the eyedropper to sample anywhere on your screen." => "انقر على القطارة لالتقاط أي لون من شاشتك.",

        // Draw
        "Pen" => "قلم",
        "Highlighter" => "قلم تظليل",
        "Eraser" => "ممحاة",
        "Undo (Ctrl+Z)" => "تراجع (Ctrl+Z)",

        // File Shelf
        "Drop files here" => "أفلت الملفات هنا",
        "Missing" => "مفقود",
        "Show in folder" | "Show in Folder" => "عرض في المجلد",
        "Share to phone" => "مشاركة للهاتف",
        "Zip" => "ضغط Zip",
        "Save all items as a .zip on the Desktop" => "حفظ جميع العناصر كملف .zip على سطح المكتب",
        "Choose files" => "اختيار ملفات",
        "Paste from the clipboard" => "لصق من الحافظة",
        "Nothing on the shelf" => "الرف فارغ حالياً",
        "Drag files, folders, text or pictures here" => "اسحب الملفات أو المجلدات أو النصوص أو الصور إلى هنا",
        "Drag out to another app \u{00B7} Double-click to open \u{00B7} Right-click to show in Explorer" => "اسحب لتطبيق آخر · نقرتين للفتح · زر أيمن للعرض في المستكشف",

        // Font Detector & Lens
        "Identify a font" => "التعرف على خط",
        "Inspect a font on screen" => "فحص خط على الشاشة",
        "Select a line of text on screen to identify its font" => "حدد سطراً نصياً على الشاشة للتعرف على نوع الخط",
        "Matches from your installed fonts:" => "الخطوط المطابقة المثبتة على جهازك:",
        "Nothing readable in that spot. Try a larger line of text." => "لم يتم العثور على نص واضح. جرب تحديد سطر أكبر.",
        "Click to copy the font name" => "انقر لنسخ اسم الخط",
        "Search the screen with Google Lens" => "البحث في الشاشة عبر عدسة جوجل",

        // Media
        "Nothing playing" => "لا يوجد تشغيل حالياً",
        "Music" => "الموسيقى",
        "Previous" => "السابق",
        "Repeat: off" => "التكرار: معطل",
        "Repeat: all" => "التكرار: الكل",
        "Repeat: one" => "التكرار: مقطع واحد",
        "Shuffle: on" => "عشوائي: مفعل",
        "Shuffle: off" => "عشوائي: معطل",
        "Lyrics (plain)" => "كلمات الأغنية",
        "Synced lyrics" => "كلمات متزامنة",
        "Looking up lyrics…" => "جاري البحث عن الكلمات…",
        "No lyrics found for this track" => "لم يتم العثور على كلمات لهذا المقطع",
        "Play a song to see its lyrics" => "شغّل مقطعاً صوتياً لعرض كلماته",

        // QR Code
        "Scan screen" => "مسح الشاشة",
        "Create" => "إنشاء",
        "No QR code found in that area" => "لم يتم العثور على رمز QR في هذه المنطقة",
        "Select a QR code on your screen" => "حدد رمز QR على شاشتك",
        "Scan a QR code on your screen" => "امسح رمز QR من على شاشتك",
        "Open" => "فتح",
        "SCANNED" => "الرموز الممسوحة",
        "Text" => "نص",
        "Link" => "رابط",
        "Contact" => "جهة اتصال",
        "Email" => "بريد إلكتروني",
        "Phone" => "هاتف",
        "SMS" => "رسالة نصية",
        "Type text or a link" => "اكتب نصاً أو رابطاً",
        "Network name (SSID)" => "اسم الشبكة (SSID)",
        "Password" => "كلمة المرور",
        "None" => "بدون حماية",
        "Full name" => "الاسم الكامل",
        "Company or title" => "الشركة أو المسمى الوظيفي",
        "To (email address)" => "إلى (البريد الإلكتروني)",
        "Subject" => "الموضوع",
        "Message" => "الرسالة",
        "Phone number" => "رقم الهاتف",
        "STYLE" => "التصميم",
        "Square" => "مربع",
        "Rounded" => "دائري الحواف",
        "Dots" => "نقاط",
        "COLOR" => "اللون",
        "Blue" => "أزرق",
        "Green" => "أخضر",
        "Purple" => "بنفسجي",
        "Red" => "أحمر",
        "Amber" => "عنبر",
        "Type on the left to build a code" => "أدخل البيانات لبناء رمز الاستجابة",
        "Check" => "فحص",
        "Decodes the preview to verify it scans" => "يفحص الرمز للتأكد من قابليته للقراءة",
        "Copy the QR image" => "نسخ صورة رمز QR",
        "Save as PNG or SVG" => "حفظ بصيغة PNG أو SVG",
        "Scans cleanly" => "يعمل بنجاح",
        "Did not decode — try Square style or shorter text" => "تعذر القراءة — جرب التصميم المربع أو نصاً أقصر",
        "Could not build that code" => "تعذر إنشاء الرمز",
        "Save failed" => "فشل الحفظ",

        // Quick Launch
        "Click + to add apps" => "انقر + لإضافة تطبيقات",
        "Pick a file" => "اختر ملفاً",
        "Pick a folder" => "اختر مجلداً",
        "Add a second Quick Launch tile to the grid" => "إضافة بلاطة تشغيل سريع ثانية إلى الشبكة",
        "Remove the second Quick Launch tile (its items are kept)" => "إزالة البلاطة الثانية (مع الاحتفاظ بعناصرها)",
        "Second tile" => "بلاطة ثانية",
        "Remove 2nd" => "إزالة الثانية",
        "Double-click to rename  \u{00B7}  Drag to reorder" => "نقرتين لإعادة التسمية  ·  اسحب للترتيب",
        "Drop apps, files or folders here, or use the bar above" => "أفلت التطبيقات أو الملفات أو المجلدات هنا أو استخدم الشريط أعلاه",
        "App, folder or URL" => "تطبيق أو مجلد أو رابط",
        "Not found" => "غير موجود",

        // System Monitor
        "CPU" => "المعالج",
        "MEMORY" => "الذاكرة",
        "Memory" => "الذاكرة",
        "RAM" => "الرام",
        "NETWORK" => "الشبكة",
        "STORAGE" => "التخزين",
        "TOP APPS" => "أعلى التطبيقات",
        "Measuring…" => "جاري القياس…",
        "Open Task Manager" => "فتح إدارة المهام",
        "Open in Explorer" => "فتح في المستكشف",
        "upload" => "رفع",

        // Voice Memo
        "Record" => "تسجيل",
        "Stop" => "إيقاف",
        "Play" => "تشغيل",
        "MEMOS" => "التسجيلات",
        "Recording" => "جاري التسجيل",
        "Ready" => "جاهز",
        "Click to stop and save" => "انقر للإيقاف والحفظ",
        "Click to record  \u{00B7}  Ctrl+Alt+R anywhere" => "انقر للتسجيل  ·  Ctrl+Alt+R في أي مكان",
        "Your memos will show up here" => "ستظهر تسجيلاتك الصوتية هنا",
        "Rename" => "إعادة تسمية",
        "Show in Explorer" => "عرض في المستكشف",
        "Move to the Recycle Bin" => "حذف إلى سلة المحذوفات",
        "Recording  \u{00B7}  click to stop" => "جاري التسجيل  ·  انقر للإيقاف",
        "Record a voice memo  \u{00B7}  Ctrl+Alt+R" => "تسجيل مذكرة صوتية  ·  Ctrl+Alt+R",
        "Open Music\\Voice memos" => "فتح مجلد المذكرات الصوتية",
        "Too short to keep" => "قصير جداً للحفظ",
        "Drag into a chat or an email" => "اسحب إلى محادثة أو بريد إلكتروني",
        "Saved to Music\\Voice memos" => "تم الحفظ في مجلد المذكرات الصوتية",
        "Could not open the microphone" => "تعذر فتح الميكروفون",
        "Could not save the memo" => "تعذر حفظ التسجيل",

        // Screenshot & Shot Editor
        "Area" => "تحديد منطقة",
        "Window" => "نافذة",
        "Fullscreen" => "شاشة كاملة",
        "Save" => "حفظ",
        "Copy" => "نسخ",
        "Edit" => "تعديل",
        "Delay" => "المهلة",
        "After capture" => "بعد الالتقاط",
        "Captures show up here." => "اللقطات ستظهر هنا.",
        "Open the Screenshots folder" => "فتح مجلد لقطات الشاشة",
        "Click to cancel" => "انقر للإلغاء",
        "Click to capture  \u{00B7}  Right click for options" => "انقر للالتقاط  ·  زر أيمن للخيارات",
        "Capture (Ctrl+Alt+S)" => "التقاط (Ctrl+Alt+S)",
        "Mark up, then copy or save" => "تعديل ثم نسخ أو حفظ",
        "Straight to the clipboard" => "مباشرة إلى الحافظة",
        "PNG in Screenshots, and copied" => "حفظ في اللقطات ونسخها",
        "Open in the editor" => "فتح في المحرر",
        "Click to edit · Right click to copy" => "انقر للتعديل · زر أيمن للنسخ",
        "Arrow" => "سهم",
        "Box" => "مستطيل",
        "Redact" => "طمس/إخفاء",
        "Size" => "الحجم",
        "Copy and close (Enter)" => "نسخ وإغلاق (Enter)",
        "Save to Screenshots (Ctrl+S)" => "حفظ في لقطات الشاشة (Ctrl+S)",
        "Copy (Ctrl+C)" => "نسخ (Ctrl+C)",
        "Saved \u{00B7} Show" => "تم الحفظ · عرض",
        "Failed" => "فشل",
        "Type, then Enter" => "اكتب ثم اضغط Enter",

        // Wallpaper
        "Next" => "التالي",
        "Folder" => "المجلد",
        "Shuffle" => "عشوائي",
        "Setting …" => "جاري التعيين…",
        "Current" => "الحالية",
        "Click to set" => "انقر للتعيين",
        "Putting the wallpaper on your desktop…" => "جاري تعيين الخلفية على سطح المكتب…",
        "No pictures found in your Pictures or Windows Web folders" => "لم يتم العثور على صور في مجلد الصور أو خلفيات ويندوز",
        "Choose folder" => "اختيار مجلد",
        "Could not set the wallpaper" => "تعذر تعيين الخلفية",
        "Browse and set your wallpaper" => "تصفح وعيّن خلفية الشاشة",
        "Pick a folder of pictures" => "اختر مجلد صور",
        "Show your custom folder plus the built-in wallpapers" => "عرض مجلدك المخصص مع خلفيات ويندوز",
        "Look for new pictures" => "البحث عن صور جديدة",
        "All" => "الكل",

        // Phone Link
        "Could not start Phone Link" => "تعذر تشغيل ربط الهاتف",
        "Mirror" => "عرض الشاشة",
        "Send" => "إرسال",
        "Waiting for the phone…" => "بانتظار اتصال الهاتف…",
        "Paired \u{00B7} waiting on Wi-Fi" => "مقترن · بانتظار الواي فاي",
        "Click to pair" => "انقر للاقتران",
        "Unavailable" => "غير متاح",
        "Pair a phone" => "اقتران هاتف",
        "Files" => "الملفات",
        "Alerts" => "التنبيهات",
        "Setup" => "الإعداد",
        "Starting…" => "جاري التشغيل…",
        "Find my phone" => "العثور على هاتفي",
        "Ring" => "رنين",
        "Stop screen mirror" => "إيقاف عرض الشاشة",
        "Mirror the phone screen (USB)" => "عرض شاشة الهاتف (USB)",
        "Lock the phone screen" => "قفل شاشة الهاتف",
        "Disconnect cleanly" => "قطع الاتصال",
        "Share" => "مشاركة",
        "Send files" => "إرسال ملفات",
        "Drop files anywhere on this page to send them" => "أفلت الملفات في أي مكان بهذه الصفحة لإرسالها",
        "RECEIVED" => "المستلمة",
        "Received files land in Downloads\\Nook" => "الملفات المستلمة تُحفظ في Downloads\\Nook",
        "Save a copy to the Desktop" => "حفظ نسخة على سطح المكتب",
        "NOTIFICATIONS" => "الإشعارات",
        "Notifications from your phone will show up here" => "ستظهر إشعارات هاتفك هنا",
        "1. Install KDE Connect on your phone" => "١. ثبّت تطبيق KDE Connect على هاتفك",
        "It's free and open-source, on Google Play, F-Droid and the App Store. Keep the phone on the same Wi-Fi as this PC." => "مجاني ومفتوح المصدر على Google Play و App Store. تأكد من اتصال الهاتف بنفس شبكة الواي فاي.",
        "2. Open the app — your PC appears as" => "٢. افتح التطبيق — سيظهر جهازك باسم",
        "Tap it and accept the pair, or scan this from the app:" => "اضغط عليه واقبل الاقتران، أو امسح هذا الرمز من التطبيق:",
        "Safety code" => "رمز الأمان",
        "PAIRING REQUEST" => "طلب اقتران",
        "Verify the code matches on your phone, then accept." => "تأكد من تطابق الرمز على هاتفك ثم اقبل.",
        "Accept" => "قبول",
        "Decline" => "رفض",
        "PAIRED DEVICES" => "الأجهزة المقترنة",
        "No phone paired yet" => "لم يتم اقتران أي هاتف بعد",
        "Pair" => "اقتران",
        "Unpair and forget this phone" => "إلغاء الاقتران ونسيان هذا الهاتف",
        "ONLINE ON YOUR WI-FI" => "متصل على نفس شبكة الواي فاي",
        "SETTINGS" => "الإعدادات",
        "Sync clipboard both ways" => "مزامنة الحافظة في الاتجاهين",
        "Pause media during phone calls" => "إيقاف الوسائط مؤقتاً أثناء المكالمات",
        "Show phone notifications" => "عرض إشعارات الهاتف",
        "Receiving" => "جاري الاستلام",
        "Sending" => "جاري الإرسال",
        "Connected" => "متصل",
        "Paired" => "مقترن",
        "Requests" => "طلبات",

        // Battery
        "Charging" => "جاري الشحن",
        "On Battery" => "على البطارية",
        "Battery" => "البطارية",

        // NotchPixel Enhanced Features & Modules
        "Weather" => "الطقس والفلك",
        "Control Center" => "مركز التحكم",
        "Focus & Tasks" => "المهام والتركيز",
        "Network Ping" => "استقرار الشبكة",
        "World Clock" => "الساعة العالمية",
        "Screen Time" => "وقت الشاشة",
        "Market Watch" => "العملات والأسواق",
        "Reminders" => "التذكيرات",
        "Unit Converter" => "محول الوحدات",
        "Lyrics" => "كلمات الأغنية",
        "Idle pill" => "وضع النوتش",
        "Clock" => "الساعة",
        "Cat Pet" => "قطة أليفة",
        "Dog Pet" => "كلب أليف",
        "Bunny Pet" => "أرنب أليف",
        "CPU & RAM" => "المعالج والرام",
        "System HUDs" => "إشعارات النظام",
        "Caps Lock ON" => "الأحرف الكبيرة: مفعل",
        "Caps Lock OFF" => "الأحرف الكبيرة: معطل",
        "Num Lock ON" => "الأرقام: مفعل",
        "Num Lock OFF" => "الأرقام: معطل",
        "Scroll Lock ON" => "التمرير: مفعل",
        "Scroll Lock OFF" => "التمرير: معطل",
        "Camera Active" => "الكاميرا قيد الاستخدام",
        "Microphone Active" => "الميكروفون قيد الاستخدام",

        // Weather & Moon Phase & Cities
        "Refresh" => "تحديث",
        "Feels" => "المحسوس",
        "Humidity" => "الرطوبة",
        "Wind" => "الرياح",
        "Range" => "النطاق",
        "24-Hour Temperature Trend" => "مسار الحرارة خلال ٢٤ ساعة",
        "Lunar Phase & Astronomy" => "طور القمر والفلك",
        "Illumination" => "نسبة الإضاءة",
        "New Moon" => "محاق",
        "Waxing Crescent" => "هلال متزايد",
        "First Quarter" => "تربيع أول",
        "Waxing Gibbous" => "أحدب متزايد",
        "Full Moon" => "بدر مكتمل",
        "Waning Gibbous" => "أحدب متناقص",
        "Last Quarter" => "تربيع أخير",
        "Waning Crescent" => "هلال متناقص",
        "Mostly Clear" => "صافي غالباً",
        "Partly Cloudy" => "غائم جزئياً",
        "Overcast" => "غائم كلياً",
        "Fog" => "ضباب",
        "Drizzle" => "رذاذ",
        "Rain" => "ممطر",
        "Snow" => "ثلوج",
        "Showers" => "زخات مطر",
        "Thunderstorm" => "عاصفة رعدية",
        "Riyadh" => "الرياض",
        "Mecca" | "Makkah" => "مكة المكرمة",
        "Medina" | "Madinah" => "المدينة المنورة",
        "Jeddah" => "جدة",
        "Dammam" => "الدمام",
        "Cairo" => "القاهرة",
        "Dubai" => "دبي",
        "Abu Dhabi" => "أبوظبي",
        "Doha" => "الدوحة",
        "Kuwait" => "الكويت",
        "Amman" => "عمّان",
        "Khartoum" => "الخرطوم",
        "Local" => "محلي",

        // Control Center
        "Display Brightness (WMI)" => "سطوع الشاشة",
        "Microphone & Audio Control" => "التحكم بالصوت والميكروفون",
        "Mic Muted" => "الميكروفون مكتوم",
        "Mic Live" => "الميكروفون نشط",
        "Vol -" => "خفض -",
        "Vol +" => "رفع +",
        "Quick Windows Controls" => "اختصارات ويندوز السريعة",
        "Night Light" => "الضوء الليلي",
        "Wi-Fi" => "واي فاي",
        "Bluetooth" => "بلوتوث",
        "Sound" => "الصوت",
        "Display" => "الشاشة",
        "Battery & Privacy Status" => "حالة البطارية والخصوصية",
        "AC Power / Desktop" => "متصل بالكهرباء",
        "Mic Active" => "الميكروفون نشط",
        "Mic Idle" => "الميكروفون خامل",
        "Cam Active" => "الكاميرا نشطة",
        "Cam Idle" => "الكاميرا خاملة",
        "Healthy" => "ممتاز",
        "Mic Off" => "الميكروفون مكتوم",

        // Focus & Tasks (Pomodoro + To-Do)
        "Done!" => "انتهى!",
        "Focus" => "تركيز",
        "Short Break" => "استراحة قصيرة",
        "Long Break" => "استراحة طويلة",
        "Sessions" => "الجلسات",
        "Tasks Done" => "المهام المنجزة",
        "Clear Done" => "مسح المنجز",
        "Add a new task..." => "أضف مهمة جديدة...",
        "Add" => "إضافة",
        "Review daily priorities" => "مراجعة الأولويات اليومية",
        "Deep focus session (25m)" => "جلسة تركيز عميق (٢٥ د)",

        // Network Ping & Uptime
        "Checking..." => "جاري الفحص...",
        "Excellent" => "ممتاز",
        "Good" => "جيد",
        "Fair" => "متوسط",
        "High Lag" => "تأخر عالي",
        "Offline" => "غير متصل",
        "Ping" => "البينج",
        "Jitter" => "التذبذب",
        "Loss" => "الفقد",
        "Website Uptime" => "حالة المواقع",
        "Live Latency Sparkline (48 samples)" => "مخطط زمن الاستجابة المباشر",
        "Target Server & Website Uptime Check" => "خادم الفحص وحالة المواقع",
        "Check All" => "فحص الكل",

        // World Clock & Time Progress
        "Time Progress" => "تقدم الوقت",
        "Day" => "اليوم",
        "Week" => "الأسبوع",
        "Month" => "الشهر",
        "Year" => "السنة",
        "Event Countdown" => "العد التنازلي للمناسبات",
        "New Year" => "رأس السنة",
        "Weekend" => "نهاية الأسبوع",
        "End of Workday" => "نهاية الدوام",
        "End of Month" => "نهاية الشهر",
        "Remaining" => "المتبقي",

        // Screen Time
        "Active Today" => "الاستخدام اليوم",
        "Top App" => "الأكثر استخداماً",
        "Apps Tracked" => "التطبيقات المرصودة",
        "Reset Today" => "تصفير اليوم",
        "Application Usage Breakdown" => "تفصيل استخدام التطبيقات",
        "Desktop" => "سطح المكتب",

        // Market Watch
        "Currency Converter (USD Base)" => "محول العملات المباشر",
        "Swap" => "تبديل",

        // Reminders
        "Triggering Now!" => "تنبيه الآن!",
        "Dismiss Alert" => "إغلاق التنبيه",
        "Active Reminders" => "التذكيرات النشطة",
        "Reminder title..." => "عنوان التذكير...",

        // Unit Converter
        "Length" => "الطول",
        "Weight" => "الوزن",
        "Temp" => "الحرارة",
        "Data" => "البيانات",
        "Speed" => "السرعة",
        "Swap Units" => "تبديل الوحدات",
        "Copy Result" => "نسخ النتيجة",
        "All Equivalent Values" => "جميع القيم المكافئة",

        // Translator, Hydration & Habits, Focus & Tools, and Notch Gestures
        "Translator" => "المترجم الفوري",
        "Translate" => "ترجمة",
        "Source Text" => "النص الأصلي",
        "Translation" => "الترجمة",
        "Translating..." => "جاري الترجمة...",
        "Recent Translations" => "أحدث الترجمات",
        "From:" => "من:",
        "To:" => "إلى:",
        "Hydration & Habits" => "الماء والعادات",
        "Cups" => "أكواب",
        "Daily Habits" => "العادات اليومية",
        "+1 Cup (250ml)" => "+1 كوب (250 مل)",
        "-1 Cup" => "-1 كوب",
        "Goal:" => "الهدف:",
        "New daily habit..." => "عادة يومية جديدة...",
        "Focus & Tools" => "التركيز والأدوات",
        "Ambient Focus Sounds" => "أصوات التركيز والهدوء",
        "Soft Rain" => "مطر هادئ",
        "Brown Noise" => "ضوضاء بنية",
        "Ocean Waves" => "أمواج البحر",
        "Focus Tone" => "نغمة تركيز",
        "Password & Key Generator" => "مولد كلمات المرور",
        "Generate" => "توليد",
        "Length:" => "الطول:",
        "Quick Decision: Dice & Coin" => "قرار سريع: النرد والعملة",
        "Roll Dice" => "رمي النرد",
        "Flip Coin" => "رمي العملة",
        "Heads" => "صورة (Heads)",
        "Tails" => "كتابة (Tails)",
        "Volume Up" => "رفع الصوت +",
        "Volume Down" => "خفض الصوت -",
        "Play / Pause" => "تشغيل / إيقاف مؤقت",

        // Prayer Times & Athkar Plugin
        "Fajr" => "الفجر",
        "Sunrise" => "الشروق",
        "Dhuhr" => "الظهر",
        "Asr" => "العصر",
        "Maghrib" => "المغرب",
        "Isha" => "العشاء",
        "Today's Timetable" => "مواقيت اليوم",
        "Athkar & Tasbeeh" => "الأذكار والتسبيح",
        "Settings & Alerts" => "الإعدادات والتنبيهات",
        "Active Remembrance (Dhikr)" => "الذكر الحالي",
        "Show on Notch" => "عرض في النوتش",
        "Selected" => "محدد",
        "Select" => "اختيار",
        "Notch Countdown Ring (Dynamic Island)" => "حلقة العد التنازلي في النوتش",
        "Show Countdown on Notch Before Azan:" => "إظهار العد التنازلي قبل الأذان بـ:",
        "Controls when the timer and ring appear on the top collapsed bar. Outside this duration, Nook displays its normal clock." => "يحدد متى يظهر المؤقت والحلقة في الشريط العلوي قبل الأذان.",
        "Athkar Remembrances in Outer Notch" => "الأذكار الدورية في النوتش الخارجي",
        "Enable Periodic Athkar in Notch" => "تفعيل الأذكار الدورية في النوتش",
        "Displays authentic Islamic remembrances in the outer notch at chosen intervals" => "يعرض أذكاراً إسلامية في النوتش الخارجي كل فترة محددة",
        "Remembrance Interval (Every how many minutes):" => "فترة تكرار الأذكار:",
        "Notch Outer Display Style:" => "طريقة عرض الذكر في النوتش:",
        "Bilingual (Dhikr + Meaning)" => "ثنائي اللغة",
        "Arabic Short" => "عربي مختصر",
        "English / Transliteration" => "إنجليزي",
        "Desktop Notification Popup" => "إشعار سطح المكتب",
        "Shows a Windows notification popup with virtue and meaning" => "يعرض إشعار ويندوز مع فضل الذكر ومعناه",
        "Play Gentle Chime on Arrival" => "تشغيل نغمة هادئة عند ظهور الذكر",
        "Plays a quiet, soothing chime when a remembrance arrives" => "يشغل نغمة تنبيه هادئة عند عرض الذكر",
        "Test Athkar Notification:" => "تجربة تنبيه الأذكار:",
        "Test on Notch Now" => "تجربة على النوتش الآن",
        "Calculation Method & Location" => "طريقة الحساب والموقع",
        "Auto-Detect Location (IP Geolocation)" => "تحديد الموقع تلقائياً",
        "Islamic Calculation Authority:" => "جهة حساب المواقيت:",
        "Umm Al-Qura (Makkah)" => "أم القرى (مكة)",
        "Egyptian Authority" => "الهيئة المصرية",
        "Muslim World League" => "رابطة العالم الإسلامي",
        "ISNA (North America)" => "أمريكا الشمالية (ISNA)",
        "Karachi" => "كراتشي",
        "Azan Alerts" => "تنبيهات الأذان",
        "Play Chime / Audio Alert at Azan" => "تشغيل تنبيه صوتي عند الأذان",
        "Plays a gentle chime when the prayer time arrives" => "يشغل تنبيهاً صوتياً عند حلول وقت الصلاة",
        "Shows a Windows notification popup when Azan arrives" => "يعرض إشعار ويندوز عند حلول وقت الأذان",
        "Test Azan Alert Now:" => "تجربة تنبيه الأذان الآن:",
        "Test Audio & Notification" => "تجربة الصوت والإشعار",
        "Reset counter" => "تصفير العداد",

        // Game Launcher Plugin
        "Games" => "الألعاب",
        "Click to open" => "انقر للفتح",
        "Game Launcher: click to open" => "مشغل الألعاب: انقر للفتح",
        "No games yet" => "لا توجد ألعاب بعد",
        "Drag a game's .exe, its shortcut (.lnk / Steam .url) or the whole game folder onto this page." => "اسحب ملف اللعبة .exe أو اختصارها أو مجلدها بالكامل إلى هذه الصفحة.",
        "Paste a game .exe, shortcut or game folder path and press Enter (or drop files here)" => "الصق مسار اللعبة أو المجلد واضغط Enter (أو أفلت الملفات هنا)",
        "Change Icon" => "تغيير الأيقونة",
        "Pick a custom icon image (.png, .jpg, .ico, .exe)" => "اختر صورة أيقونة مخصصة (.png, .jpg, .ico, .exe)",
        "Restore original .exe icon" => "استعادة أيقونة اللعبة الأصلية",
        "Rename game" => "إعادة تسمية اللعبة",
        "Delete game" => "حذف اللعبة",
        "Close menu" => "إغلاق القائمة",
        "Type new name and press Enter" => "اكتب الاسم الجديد واضغط Enter",

        // Dawwar Plugin
        "Dawwar Background Wheel & Settings" => "عجلة وإعدادات دوّار في الخلفية",
        "Running in background | Drag any file & hold Shift, Ctrl, Space, or Alt" => "يعمل في الخلفية | اسحب أي ملف واضغط Shift أو Ctrl أو Space أو Alt",
        "Wheel is currently paused | Toggle Active to enable background hooks" => "العجلة متوقفة مؤقتاً | فعّل خيار نشط لتشغيلها",
        "Shift + Drag" => "Shift + سحب",
        "Convert Format (PDF, Video, Image)" => "تحويل الصيغة (PDF، فيديو، صور)",
        "Ctrl + Drag" => "Ctrl + سحب",
        "Tools (Compress, Merge, Transcribe)" => "أدوات (ضغط، دمج، تفريغ صوتي)",
        "Space + Drag" => "Space + سحب",
        "Actions (Copy Path, Move, Terminal)" => "إجراءات (نسخ المسار، نقل، الطرفية)",
        "Alt + Drag (AI)" => "Alt + سحب (ذكاء اصطناعي)",
        "Enabled (Summarize, Translate, Slides)" => "مفعل (تلخيص، ترجمة، عروض تقديمية)",
        "Off (Enable in Dawwar Settings)" => "معطل (فعّله من إعدادات دوّار)",
        "Appearance (Applies to Dawwar Wheel & Windows)" => "المظهر (ينطبق على عجلة ونوافذ دوّار)",
        "Theme Mode" => "وضع السمة",
        "Accent Palette" => "لوحة الألوان",
        "Emerald" => "زمردي",
        "Night" => "ليلي",
        "Garnet" => "عقيقي",
        "Oud" => "عود",
        "Plum" => "برقوقي",
        "Graphite" => "جرافيت",
        "Glass Wheel & Surface Effect" => "تأثير الزجاج للعجلة والواجهة",
        "Translucent acrylic glass backdrop behind the radial wheel" => "خلفية زجاجية شفافة خلف العجلة الدائرية",
        "Wheel Behavior & Startup" => "سلوك العجلة وبدء التشغيل",
        "Wheel Navigation Tick Sound" => "صوت نقرات التنقل في العجلة",
        "Play subtle tactile click when hovering across wheel slices" => "تشغيل نقرة خفيفة عند التمرير بين خيارات العجلة",
        "Launch Dawwar with Windows" => "تشغيل دوّار مع بدء ويندوز",
        "Also register Dawwar in Windows startup (in addition to Nook auto-start)" => "تسجيل دوّار في بدء تشغيل ويندوز أيضاً",
        "Automatic Update Checks" => "فحص التحديثات تلقائياً",
        "Check daily for signed Dawwar updates" => "التحقق يومياً من وجود تحديثات لبرنامج دوّار",
        "AI Wheel (Hold Alt while dragging)" => "عجلة الذكاء الاصطناعي (اضغط Alt أثناء السحب)",
        "Summarize, translate, generate slides (PPTX), website, or video script" => "تلخيص وترجمة وإنشاء عروض تقديمية (PPTX) أو صفحة ويب",
        "AI Provider" => "مزود الذكاء الاصطناعي",
        "OpenRouter Model" => "نموذج OpenRouter",
        "Key Installed: Ready for Alt + Drag AI Wheel" => "المفتاح مثبت: جاهز لعجلة الذكاء الاصطناعي (Alt + سحب)",
        "No Key: Paste API key below to activate Alt wheel" => "لا يوجد مفتاح: الصق مفتاح API أدناه لتفعيل عجلة Alt",
        "Paste OpenRouter Key here (starts with sk-or-...)" => "الصق مفتاح OpenRouter هنا (يبدأ بـ sk-or-...)",
        "Save Key & Activate" => "حفظ المفتاح وتفعيله",
        "Remove Key" => "إزالة المفتاح",
        "Get API Key" => "الحصول على مفتاح",
        "Claude Code Model" => "نموذج Claude Code",
        "Uses your installed OpenAI Codex CLI with its default model in read-only mode." => "يستخدم أداة OpenAI Codex CLI المثبتة لديك.",
        "Offline Speech-to-Text (Hold Ctrl on Audio/Video)" => "تحويل الصوت إلى نص بدون إنترنت (Ctrl + سحب)",
        "Transcription Model" => "نموذج التفريغ الصوتي",
        "Runs 100% offline on your PC using ONNX Runtime. Hold Ctrl while dragging audio or video to transcribe." => "يعمل بدون إنترنت على جهازك عبر ONNX. اضغط Ctrl أثناء سحب ملف صوتي أو فيديو.",
        "Direct Notch Drop Converter" => "محول الملفات المباشر بالسحب والإفلات",
        "Open Sample Files" => "فتح ملفات تجريبية",
        "Drop any file (PDF, image, video, audio, Word, CSV, SRT) onto the Dawwar tile or this notch window to convert it immediately!" => "أفلت أي ملف (PDF، صورة، فيديو، صوت، وورد) على بلاطة دوّار أو هذه النافذة لتحويله فوراً!",
        "Dawwar Global Wheel Shortcuts (Drag Any File in Windows)" => "اختصارات عجلة دوّار الشاملة (عند سحب أي ملف في ويندوز)",
        "1. Hold [Shift] while dragging" => "١. اضغط [Shift] أثناء السحب",
        "Convert format: PDF to Word/JPG/PNG, Video to MP4/MP3/GIF, Image to PNG/JPG/PDF" => "تحويل الصيغ: PDF إلى وورد/صور، فيديو إلى MP4/MP3/GIF، صور إلى PNG/JPG/PDF",
        "2. Hold [Ctrl] while dragging" => "٢. اضغط [Ctrl] أثناء السحب",
        "Smart Tools: Compress, Half-Size, Offline Speech-to-Text, Merge/Split PDF, Fix Arabic UTF-8" => "أدوات ذكية: ضغط، تصغير الحجم، تفريغ صوتي، دمج/تقسيم PDF، إصلاح الترميز العربي",
        "3. Hold [Space] while dragging" => "٣. اضغط [Space] أثناء السحب",
        "Quick Actions: Copy Path, Copy/Move To Folder, Open Terminal Here, Show in Explorer" => "إجراءات سريعة: نسخ المسار، نسخ/نقل لمجلد، فتح الطرفية، عرض في المستكشف",
        "4. Hold [Alt] while dragging" => "٤. اضغط [Alt] أثناء السحب",
        "AI Wheel: Summarize to Word, Translate AR/EN, Presentation Slides (PPTX), Website, Video Script" => "عجلة الذكاء الاصطناعي: تلخيص لوورد، ترجمة عربي/إنجليزي، شرائح بوربوينت، موقع ويب",
        "Open Dawwar Native Window" => "فتح نافذة دوّار الأصلية",
        "Open Dawwar Sample Files" => "فتح الملفات التجريبية لدوّار",
        "Restart Dawwar Background Engine" => "إعادة تشغيل محرك دوّار في الخلفية",
        "Select an action below to run on your dropped file(s)" => "اختر إجراءً أدناه لتنفيذه على الملفات",
        "To PNG" => "إلى PNG",
        "To JPG" => "إلى JPG",
        "To PDF" => "إلى PDF",
        "Merge to PDF" => "دمج في PDF",
        "Compress" => "ضغط",
        "Half Size" => "نصف الحجم",
        "Strip GPS/Meta" => "إزالة بيانات الموقع",
        "Rotate 90 deg" => "تدوير ٩٠ درجة",
        "To Word (DOCX)" => "إلى وورد (DOCX)",
        "Pages to JPG" => "الصفحات إلى JPG",
        "Pages to PNG" => "الصفحات إلى PNG",
        "Extract Text" => "استخراج النص",
        "Compress PDF" => "ضغط PDF",
        "Strong Compress" => "ضغط قوي",
        "Split Pages" => "تقسيم الصفحات",
        "Merge PDFs" => "دمج ملفات PDF",
        "Merge Folder PDFs" => "دمج PDF في المجلد",
        "Extract MP3" => "استخراج MP3",
        "To MP4" => "إلى MP4",
        "To GIF" => "إلى GIF",
        "For Messaging" => "للمراسلة",
        "Small Size" => "حجم صغير",
        "Transcribe Text" => "تفريغ نصي",
        "Subtitles (SRT)" => "ترجمة (SRT)",
        "To MP3" => "إلى MP3",
        "To WAV" => "إلى WAV",
        "To M4A" => "إلى M4A",
        "Light Audio" => "صوت خفيف",
        "To TXT" => "إلى TXT",
        "Fix Arabic UTF-8" => "إصلاح الترميز العربي",
        "Digits 123" => "أرقام 123",
        "Arabic Digits" => "أرقام عربية",
        "Clean Arabic" => "تنظيف النص العربي",
        "Excel Ready" => "جاهز لإكسل",
        "Extract Archive" => "فك الضغط",
        "Compress ZIP" => "ضغط ZIP",
        "Copy Path" => "نسخ المسار",

        // Fallback: return unchanged
        other => other,
    }
}
