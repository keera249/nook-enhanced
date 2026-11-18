//! Font matching on Linux (uses Lens neural fontnet + `fc-list`).

use crate::capture::Frame;
use crate::{PlatformError, Result};
use std::process::Command;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FontStyle {
    Regular,
    Bold,
    Italic,
}

impl FontStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Regular => "Regular",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FontMatch {
    pub family: String,
    pub style: FontStyle,
    pub score: f32,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Installed,
    Lens,
    Both,
}

#[derive(Clone, Debug)]
pub struct Detection {
    pub text: String,
    pub matches: Vec<FontMatch>,
    pub sample: Frame,
}

pub fn families() -> Vec<String> {
    let Ok(out) = Command::new("fc-list").args([":", "family"]).output() else {
        return Vec::new();
    };
    let mut list: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .flat_map(|l| l.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    list.sort();
    list.dedup();
    list
}

pub fn detect(frame: &Frame, top: usize) -> Result<Detection> {
    let dir = crate::fontnet::model_dir()
        .ok_or_else(|| PlatformError::Other("fontnet model not installed".into()))?;
    let preds = crate::fontnet::classify(&dir, std::slice::from_ref(frame), top)?;
    let matches = preds
        .into_iter()
        .map(|p| FontMatch {
            family: p.family,
            style: FontStyle::Regular,
            score: p.prob,
            source: Source::Lens,
        })
        .collect();
    Ok(Detection {
        text: String::new(),
        matches,
        sample: frame.clone(),
    })
}
