//! Arabic text shaping for egui and Nook.
//!
//! Egui by default does not perform OpenType GSUB cursive shaping.
//! This module converts Arabic Unicode runs (U+0600..U+06FF and presentation forms)
//! into contextual Presentation Forms-B (Isolated, Initial, Medial, Final, and Lam-Alif
//! ligatures) so that Arabic script renders seamlessly connected, cursive, and legible.

/// Checks whether a character belongs to the Arabic script or presentation forms.
pub fn is_arabic(c: char) -> bool {
    matches!(c,
        '\u{0600}'..='\u{06FF}' | // Arabic
        '\u{0750}'..='\u{077F}' | // Arabic Supplement
        '\u{08A0}'..='\u{08FF}' | // Arabic Extended-A
        '\u{FB50}'..='\u{FDFF}' | // Arabic Presentation Forms-A
        '\u{FE70}'..='\u{FEFC}'   // Arabic Presentation Forms-B
    )
}

/// Checks whether a character is an Arabic diacritic (Tashkeel / Harakat).
pub fn is_tashkeel(c: char) -> bool {
    matches!(c,
        '\u{064B}'..='\u{065F}' | // Fathatan, Dammatan, Kasratan, Fatha, Damma, Kasra, Shadda, Sukun
        '\u{0670}'               // Dagger Alef / Superscript Alef
    )
}

/// Returns true if the string contains any Arabic characters.
pub fn contains_arabic(s: &str) -> bool {
    s.chars().any(is_arabic)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum JoiningType {
    Transparent,
    NonJoining,
    RightJoining,
    DualJoining,
}

#[derive(Clone, Copy, Debug)]
struct ArabicCharForms {
    isolated: char,
    final_: char,
    initial: char,
    medial: char,
    joining: JoiningType,
}

impl ArabicCharForms {
    const fn new(isolated: char, final_: char, initial: char, medial: char, joining: JoiningType) -> Self {
        Self { isolated, final_, initial, medial, joining }
    }

    const fn right_only(isolated: char, final_: char) -> Self {
        Self { isolated, final_, initial: isolated, medial: final_, joining: JoiningType::RightJoining }
    }

    const fn non_joining(c: char) -> Self {
        Self { isolated: c, final_: c, initial: c, medial: c, joining: JoiningType::NonJoining }
    }
}

fn char_forms(c: char) -> Option<ArabicCharForms> {
    if is_tashkeel(c) {
        return Some(ArabicCharForms::new(c, c, c, c, JoiningType::Transparent));
    }

    let forms = match c {
        // Hamza
        '\u{0621}' => ArabicCharForms::non_joining('\u{FE80}'),
        // Alef with Madda Above
        '\u{0622}' => ArabicCharForms::right_only('\u{FE81}', '\u{FE82}'),
        // Alef with Hamza Above
        '\u{0623}' => ArabicCharForms::right_only('\u{FE83}', '\u{FE84}'),
        // Waw with Hamza Above
        '\u{0624}' => ArabicCharForms::right_only('\u{FE85}', '\u{FE86}'),
        // Alef with Hamza Below
        '\u{0625}' => ArabicCharForms::right_only('\u{FE87}', '\u{FE88}'),
        // Yeh with Hamza Above
        '\u{0626}' => ArabicCharForms::new('\u{FE89}', '\u{FE8A}', '\u{FE8B}', '\u{FE8C}', JoiningType::DualJoining),
        // Alef
        '\u{0627}' => ArabicCharForms::right_only('\u{FE8D}', '\u{FE8E}'),
        // Beh
        '\u{0628}' => ArabicCharForms::new('\u{FE8F}', '\u{FE90}', '\u{FE91}', '\u{FE92}', JoiningType::DualJoining),
        // Teh Marbuta
        '\u{0629}' => ArabicCharForms::right_only('\u{FE93}', '\u{FE94}'),
        // Teh
        '\u{062A}' => ArabicCharForms::new('\u{FE95}', '\u{FE96}', '\u{FE97}', '\u{FE98}', JoiningType::DualJoining),
        // Theh
        '\u{062B}' => ArabicCharForms::new('\u{FE99}', '\u{FE9A}', '\u{FE9B}', '\u{FE9C}', JoiningType::DualJoining),
        // Jeem
        '\u{062C}' => ArabicCharForms::new('\u{FE9D}', '\u{FE9E}', '\u{FE9F}', '\u{FEA0}', JoiningType::DualJoining),
        // Hah
        '\u{062D}' => ArabicCharForms::new('\u{FEA1}', '\u{FEA2}', '\u{FEA3}', '\u{FEA4}', JoiningType::DualJoining),
        // Khah
        '\u{062E}' => ArabicCharForms::new('\u{FEA5}', '\u{FEA6}', '\u{FEA7}', '\u{FEA8}', JoiningType::DualJoining),
        // Dal
        '\u{062F}' => ArabicCharForms::right_only('\u{FEA9}', '\u{FEAA}'),
        // Thal
        '\u{0630}' => ArabicCharForms::right_only('\u{FEAB}', '\u{FEAC}'),
        // Reh
        '\u{0631}' => ArabicCharForms::right_only('\u{FEAD}', '\u{FEAE}'),
        // Zain
        '\u{0632}' => ArabicCharForms::right_only('\u{FEAF}', '\u{FEB0}'),
        // Seen
        '\u{0633}' => ArabicCharForms::new('\u{FEB1}', '\u{FEB2}', '\u{FEB3}', '\u{FEB4}', JoiningType::DualJoining),
        // Sheen
        '\u{0634}' => ArabicCharForms::new('\u{FEB5}', '\u{FEB6}', '\u{FEB7}', '\u{FEB8}', JoiningType::DualJoining),
        // Sad
        '\u{0635}' => ArabicCharForms::new('\u{FEB9}', '\u{FEBA}', '\u{FEBB}', '\u{FEBC}', JoiningType::DualJoining),
        // Dad
        '\u{0636}' => ArabicCharForms::new('\u{FEBD}', '\u{FEBE}', '\u{FEBF}', '\u{FEC0}', JoiningType::DualJoining),
        // Tah
        '\u{0637}' => ArabicCharForms::new('\u{FEC1}', '\u{FEC2}', '\u{FEC3}', '\u{FEC4}', JoiningType::DualJoining),
        // Zah
        '\u{0638}' => ArabicCharForms::new('\u{FEC5}', '\u{FEC6}', '\u{FEC7}', '\u{FEC8}', JoiningType::DualJoining),
        // Ain
        '\u{0639}' => ArabicCharForms::new('\u{FEC9}', '\u{FECA}', '\u{FECB}', '\u{FECC}', JoiningType::DualJoining),
        // Ghain
        '\u{063A}' => ArabicCharForms::new('\u{FECD}', '\u{FECE}', '\u{FECF}', '\u{FED0}', JoiningType::DualJoining),
        // Tatweel
        '\u{0640}' => ArabicCharForms::new('\u{0640}', '\u{0640}', '\u{0640}', '\u{0640}', JoiningType::DualJoining),
        // Feh
        '\u{0641}' => ArabicCharForms::new('\u{FED1}', '\u{FED2}', '\u{FED3}', '\u{FED4}', JoiningType::DualJoining),
        // Qaf
        '\u{0642}' => ArabicCharForms::new('\u{FED5}', '\u{FED6}', '\u{FED7}', '\u{FED8}', JoiningType::DualJoining),
        // Kaf
        '\u{0643}' => ArabicCharForms::new('\u{FED9}', '\u{FEDA}', '\u{FEDB}', '\u{FEDC}', JoiningType::DualJoining),
        // Lam
        '\u{0644}' => ArabicCharForms::new('\u{FEDD}', '\u{FEDE}', '\u{FEDF}', '\u{FEE0}', JoiningType::DualJoining),
        // Meem
        '\u{0645}' => ArabicCharForms::new('\u{FEE1}', '\u{FEE2}', '\u{FEE3}', '\u{FEE4}', JoiningType::DualJoining),
        // Noon
        '\u{0646}' => ArabicCharForms::new('\u{FEE5}', '\u{FEE6}', '\u{FEE7}', '\u{FEE8}', JoiningType::DualJoining),
        // Heh
        '\u{0647}' => ArabicCharForms::new('\u{FEE9}', '\u{FEEA}', '\u{FEEB}', '\u{FEEC}', JoiningType::DualJoining),
        // Waw
        '\u{0648}' => ArabicCharForms::right_only('\u{FEED}', '\u{FEEE}'),
        // Alef Maksura
        '\u{0649}' => ArabicCharForms::right_only('\u{FEEF}', '\u{FEF0}'),
        // Yeh
        '\u{064A}' => ArabicCharForms::new('\u{FEF1}', '\u{FEF2}', '\u{FEF3}', '\u{FEF4}', JoiningType::DualJoining),

        // Extended Persian / Urdu letters commonly occurring
        '\u{067E}' => ArabicCharForms::new('\u{FB56}', '\u{FB57}', '\u{FB58}', '\u{FB59}', JoiningType::DualJoining), // Peh
        '\u{0686}' => ArabicCharForms::new('\u{FB7A}', '\u{FB7B}', '\u{FB7C}', '\u{FB7D}', JoiningType::DualJoining), // Tcheh
        '\u{06A4}' => ArabicCharForms::new('\u{FB6A}', '\u{FB6B}', '\u{FB6C}', '\u{FB6D}', JoiningType::DualJoining), // Veh
        '\u{06AF}' => ArabicCharForms::new('\u{FB92}', '\u{FB93}', '\u{FB94}', '\u{FB95}', JoiningType::DualJoining), // Gaf

        _ => return None,
    };
    Some(forms)
}

fn lam_alif_ligature(alef: char, connects_prev: bool) -> Option<char> {
    match alef {
        '\u{0622}' => Some(if connects_prev { '\u{FEF6}' } else { '\u{FEF5}' }), // Alef Madda
        '\u{0623}' => Some(if connects_prev { '\u{FEF8}' } else { '\u{FEF7}' }), // Alef Hamza Above
        '\u{0625}' => Some(if connects_prev { '\u{FEFA}' } else { '\u{FEF9}' }), // Alef Hamza Below
        '\u{0627}' => Some(if connects_prev { '\u{FEFC}' } else { '\u{FEFB}' }), // Plain Alef
        _ => None,
    }
}

/// Shapes an Arabic word/segment in logical reading order into Presentation Forms-B.
/// Tashkeel (harakat) are preserved directly following their respective base characters.
fn shape_arabic_run(text: &str) -> Vec<String> {
    let raw_chars: Vec<char> = text.chars().collect();
    let n = raw_chars.len();
    if n == 0 {
        return Vec::new();
    }

    // First group each base character with any following Tashkeel
    struct CharGroup {
        base: char,
        tashkeel: Vec<char>,
        forms: Option<ArabicCharForms>,
    }

    let mut groups: Vec<CharGroup> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let c = raw_chars[i];
        if is_tashkeel(c) {
            if let Some(last) = groups.last_mut() {
                last.tashkeel.push(c);
            } else {
                groups.push(CharGroup {
                    base: c,
                    tashkeel: Vec::new(),
                    forms: char_forms(c),
                });
            }
            i += 1;
            continue;
        }

        groups.push(CharGroup {
            base: c,
            tashkeel: Vec::new(),
            forms: char_forms(c),
        });
        i += 1;
    }

    let mut shaped_units: Vec<String> = Vec::with_capacity(groups.len());
    let glen = groups.len();
    let mut gi = 0;

    while gi < glen {
        let cur = &groups[gi];
        let Some(cur_forms) = cur.forms else {
            // Non-Arabic character
            let mut s = String::new();
            s.push(cur.base);
            for &t in &cur.tashkeel {
                s.push(t);
            }
            shaped_units.push(s);
            gi += 1;
            continue;
        };

        // Determine if previous non-transparent connects to current
        let mut prev_connects = false;
        let mut pi = gi;
        while pi > 0 {
            pi -= 1;
            if let Some(pf) = groups[pi].forms {
                if pf.joining != JoiningType::Transparent {
                    prev_connects = pf.joining == JoiningType::DualJoining;
                    break;
                }
            } else {
                break;
            }
        }

        // Check for Lam-Alif ligature: current is Lam (0x0644) and next base is Alef
        if cur.base == '\u{0644}' && gi + 1 < glen {
            let next_base = groups[gi + 1].base;
            if let Some(lig) = lam_alif_ligature(next_base, prev_connects) {
                let mut s = String::new();
                s.push(lig);
                for &t in &cur.tashkeel {
                    s.push(t);
                }
                for &t in &groups[gi + 1].tashkeel {
                    s.push(t);
                }
                shaped_units.push(s);
                gi += 2; // skip both Lam and Alef
                continue;
            }
        }

        // Determine if next non-transparent connects back to current
        let mut next_connects = false;
        let mut ni = gi + 1;
        while ni < glen {
            if let Some(nf) = groups[ni].forms {
                if nf.joining != JoiningType::Transparent {
                    next_connects = nf.joining == JoiningType::DualJoining
                        || nf.joining == JoiningType::RightJoining;
                    break;
                }
            } else {
                break;
            }
            ni += 1;
        }

        let shaped_char = match cur_forms.joining {
            JoiningType::RightJoining => {
                if prev_connects {
                    cur_forms.final_
                } else {
                    cur_forms.isolated
                }
            }
            JoiningType::DualJoining => {
                if prev_connects && next_connects {
                    cur_forms.medial
                } else if prev_connects {
                    cur_forms.final_
                } else if next_connects {
                    cur_forms.initial
                } else {
                    cur_forms.isolated
                }
            }
            JoiningType::NonJoining | JoiningType::Transparent => cur_forms.isolated,
        };

        let mut s = String::new();
        s.push(shaped_char);
        for &t in &cur.tashkeel {
            s.push(t);
        }
        shaped_units.push(s);
        gi += 1;
    }

    shaped_units
}

/// Shapes Arabic characters within text into connected Presentation Forms-B.
/// Non-Arabic text (English, digits, symbols) remains completely untouched.
pub fn shape_bidi(text: &str) -> String {
    if !contains_arabic(text) {
        return text.to_owned();
    }

    let mut out = String::with_capacity(text.len() * 2);
    let mut lines = text.split('\n').peekable();

    while let Some(line) = lines.next() {
        if !contains_arabic(line) {
            out.push_str(line);
            if lines.peek().is_some() {
                out.push('\n');
            }
            continue;
        }

        let mut cur_arabic = false;
        let mut cur_buf = String::new();

        let flush_buf = |buf: &mut String, is_ar: bool, target: &mut String| {
            if buf.is_empty() {
                return;
            }
            if is_ar {
                let units = shape_arabic_run(buf);
                for u in units {
                    target.push_str(&u);
                }
            } else {
                target.push_str(buf);
            }
            buf.clear();
        };

        for c in line.chars() {
            let is_ar = is_arabic(c) || is_tashkeel(c);
            if cur_buf.is_empty() {
                cur_arabic = is_ar;
                cur_buf.push(c);
            } else if is_ar == cur_arabic {
                cur_buf.push(c);
            } else {
                flush_buf(&mut cur_buf, cur_arabic, &mut out);
                cur_arabic = is_ar;
                cur_buf.push(c);
            }
        }
        flush_buf(&mut cur_buf, cur_arabic, &mut out);

        if lines.peek().is_some() {
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_subhan() {
        let input = "سبحان";
        let shaped = shape_bidi(input);
        let chars: Vec<char> = shaped.chars().collect();
        // Logical forward order: Seen Initial on left, connecting to Beh Medial, etc.
        assert_eq!(chars[0], '\u{FEB3}'); // Seen Initial
        assert_eq!(chars[1], '\u{FE92}'); // Beh Medial
        assert_eq!(chars[2], '\u{FEA4}'); // Hah Medial
        assert_eq!(chars[3], '\u{FE8E}'); // Alef Final
        assert_eq!(chars[4], '\u{FEE5}'); // Noon Isolated
    }

    #[test]
    fn shapes_lam_alif() {
        let input = "لا";
        let shaped = shape_bidi(input);
        let chars: Vec<char> = shaped.chars().collect();
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0], '\u{FEFB}'); // Lam-Alif Isolated
    }

    #[test]
    fn preserves_non_arabic() {
        let input = "Hello 123 - Timer";
        let shaped = shape_bidi(input);
        assert_eq!(shaped, input);
    }

    #[test]
    fn mixed_english_and_arabic() {
        let input = "Dhikr: سبحان";
        let shaped = shape_bidi(input);
        assert!(shaped.starts_with("Dhikr: "));
        let rest = &shaped["Dhikr: ".len()..];
        let chars: Vec<char> = rest.chars().collect();
        assert_eq!(chars[0], '\u{FEB3}'); // Seen Initial
        assert_eq!(chars[4], '\u{FEE5}'); // Noon Isolated
    }

    #[test]
    fn shapes_allah() {
        let input = "الله";
        let shaped = shape_bidi(input);
        let chars: Vec<char> = shaped.chars().collect();
        assert_eq!(chars[0], '\u{FE8D}'); // Alef Isolated
        assert_eq!(chars[1], '\u{FEDF}'); // Lam Initial
        assert_eq!(chars[2], '\u{FEE0}'); // Lam Medial
        assert_eq!(chars[3], '\u{FEEA}'); // Heh Final
    }

    #[test]
    fn shapes_arabic_with_time() {
        let input = "صلاة الفجر 05:12";
        let shaped = shape_bidi(input);
        assert!(shaped.contains("05:12"));
        assert!(shaped.contains('\u{FEAE}')); // Reh Final in الفجر
    }
}
