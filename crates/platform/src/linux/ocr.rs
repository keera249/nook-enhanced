//! OCR via `tesseract` CLI on Linux.

use crate::capture::Frame;
use crate::{PlatformError, Result};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrLanguage {
    pub tag: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct OcrWord {
    pub text: String,
    pub line: usize,
    pub rect: (f32, f32, f32, f32),
}

#[derive(Clone, Debug)]
pub struct OcrText {
    pub lines: Vec<String>,
    pub words: Vec<OcrWord>,
    pub language: String,
}

impl OcrText {
    pub fn joined(&self, single_line: bool) -> String {
        if single_line {
            self.lines.join(" ")
        } else {
            self.lines.join("\n")
        }
    }
}

pub fn languages() -> Vec<OcrLanguage> {
    let Ok(out) = Command::new("tesseract").arg("--list-langs").output() else {
        return vec![OcrLanguage {
            tag: "eng".into(),
            name: "English".into(),
        }];
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let langs: Vec<OcrLanguage> = text
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|tag| OcrLanguage {
            tag: tag.to_owned(),
            name: tag.to_owned(),
        })
        .collect();
    if langs.is_empty() {
        vec![OcrLanguage {
            tag: "eng".into(),
            name: "English".into(),
        }]
    } else {
        langs
    }
}

pub fn recognize(frame: &Frame, language: Option<&str>) -> Result<OcrText> {
    let png = frame.encode_png()?;
    let tmp = std::env::temp_dir().join(format!("nook-ocr-{}.png", std::process::id()));
    std::fs::write(&tmp, png).map_err(|e| PlatformError::Other(e.to_string()))?;
    let lang = language.unwrap_or("eng");
    let out = Command::new("tesseract")
        .arg(&tmp)
        .arg("stdout")
        .args(["-l", lang])
        .output();
    let _ = std::fs::remove_file(&tmp);
    let out = out.map_err(|e| PlatformError::Other(format!("tesseract: {e}")))?;
    if !out.status.success() {
        return Err(PlatformError::Other("tesseract failed".into()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    let mut words = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        for w in line.split_whitespace() {
            words.push(OcrWord {
                text: w.to_owned(),
                line: idx,
                rect: (0.0, idx as f32 * 20.0, 40.0, 18.0),
            });
        }
    }
    Ok(OcrText {
        lines,
        words,
        language: lang.to_owned(),
    })
}
