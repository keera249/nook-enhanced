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
        return key.to_owned();
    }
    let ar = translate_ar(key);
    shape_bidi(ar)
}

/// Translates a key string into Arabic raw form.
pub fn translate_ar(key: &str) -> &str {
    match key {
        // Module Names & Titles
        "Timer" => "المؤقت",
        "Coffee" => "مانع السكون",
        "Media" => "مشغل الوسائط",
        "Notes" => "الملاحظات",
        "Clipboard" => "الحافظة",
        "Screenshot" => "لقطة الشاشة",
        "Screen Capture" => "لقطة الشاشة",
        "Lens" => "العدسة",
        "Calendar" => "التقويم",
        "OCR" => "قارئ النصوص",
        "Text Extractor" => "قارئ النصوص",
        "Color Picker" => "منتقي الألوان",
        "Draw" => "لوحة الرسم",
        "Phone Link" => "ربط الهاتف",
        "File Shelf" => "رف الملفات",
        "Font Detector" => "كاشف الخطوط",
        "Wallpaper" => "الخلفيات",
        "Calculator" => "الآلة الحاسبة",
        "System Monitor" => "مراقب النظام",
        "System monitor" => "مراقب النظام",
        "Quick Launch" => "تشغيل سريع",
        "QR Code" => "رمز الاستجابة",
        "Voice Memo" => "مذكرات صوتية",
        "Voice memo" => "مذكرات صوتية",
        "Plugins" => "الإضافات",
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
        "Settings" => "الإعدادات",

        // Settings - Sections
        "Appearance" => "المظهر",
        "Behavior" => "السلوك",
        "Customize" => "تخصيص متقدم",
        "Advanced" => "خيارات متقدمة",

        // Settings - Appearance Controls
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
        "Edit modules" => "ترتيب الوحدات",
        "Done" => "تم",
        "Quit Nook" => "إنهاء البرنامج",
        "Quit" => "إنهاء",
        "Exit" => "خروج",
        "Pin" => "تثبيت",
        "Unpin" => "إلغاء التثبيت",
        "Close" => "إغلاق",
        "Back" => "رجوع",

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

        // Coffee
        "Always" => "دائماً",
        "Awake" => "نشط",
        "Click to change how long" => "انقر لتغيير المدة",

        // Notes
        "New note" => "ملاحظة جديدة",
        "Start typing. The first line is the title." => "ابدأ بالكتابة. السطر الأول هو العنوان.",
        "Write something" => "اكتب شيئاً...",
        "No notes. Press + to start one." => "لا توجد ملاحظات. اضغط + للبدء.",
        "Editing" => "تعديل",
        "Move this note to the Recycle Bin" => "نقل الملاحظة إلى سلة المحذوفات",

        // Clipboard
        "Copied" => "تم النسخ",
        "Copy something" => "انسخ نصاً أو صورة",
        "Show all" => "عرض الكل",
        "Clear history" => "مسح السجل",
        "Nothing copied yet. Text and images you copy show up here." => "لا يوجد شيء منسوخ بعد. ما تنسخه سيظهر هنا.",
        "Image" => "صورة",
        "Remove" => "إزالة",

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
        "Delete" => "حذف",
        "Add: 14:30 Dentist" => "إضافة: 14:30 موعد الطبيب",
        "January" => "يناير",
        "February" => "فبراير",
        "March" => "مارس",
        "April" => "أبريل",
        "May" => "مايو",
        "June" => "يونيو",
        "July" => "يوليو",
        "August" => "أغسطس",
        "September" => "سبتمبر",
        "October" => "أكتوبر",
        "November" => "نوفمبر",
        "December" => "ديسمبر",
        "Monday" => "الإثنين",
        "Tuesday" => "الثلاثاء",
        "Wednesday" => "الأربعاء",
        "Thursday" => "الخميس",
        "Friday" => "الجمعة",
        "Saturday" => "السبت",
        "Sunday" => "الأحد",

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

        // Screenshot
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

        // Wallpaper
        "Next" => "التالي",
        "Folder" => "المجلد",
        "Shuffle" => "عشوائي",

        // Battery
        "Charging" => "جاري الشحن",
        "Battery" => "البطارية",

        // Fallback: return unchanged
        other => other,
    }
}
