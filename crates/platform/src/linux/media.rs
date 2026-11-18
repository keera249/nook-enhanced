//! Media session watcher & controls on Linux via MPRIS (`playerctl`).

use crate::Result;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub const ART_SIZE: u32 = 180;

#[derive(Clone, Debug, PartialEq)]
pub struct Art {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct NowPlaying {
    pub source: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub can_previous: bool,
    pub can_next: bool,
    pub can_toggle: bool,
    pub timeline: Option<(f64, f64)>,
    pub captured: Instant,
    pub art: Option<Arc<Art>>,
}

impl NowPlaying {
    pub fn position(&self) -> Option<(f64, f64)> {
        let (pos, dur) = self.timeline?;
        if self.playing {
            let elapsed = self.captured.elapsed().as_secs_f64();
            Some(((pos + elapsed).min(dur), dur))
        } else {
            Some((pos, dur))
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    Seek(f64),
}

enum Msg {
    Cmd(MediaCommand),
    Stop,
}

pub struct MediaWatcher {
    tx: Sender<Msg>,
    rx: Receiver<Option<NowPlaying>>,
}

impl MediaWatcher {
    pub fn spawn(on_change: impl Fn() + Send + 'static) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let (state_tx, state_rx) = mpsc::channel::<Option<NowPlaying>>();

        thread::spawn(move || {
            let mut last_key = String::new();
            let mut cached_art: Option<(String, Option<Arc<Art>>)> = None;
            let mut last_had_media = false;

            loop {
                while let Ok(msg) = cmd_rx.try_recv() {
                    match msg {
                        Msg::Stop => return,
                        Msg::Cmd(cmd) => exec_cmd(cmd),
                    }
                }

                let np = poll_playerctl(&mut cached_art);
                let key = np
                    .as_ref()
                    .map(|n| format!("{}|{}|{}|{}", n.source, n.title, n.artist, n.playing))
                    .unwrap_or_default();

                if key != last_key || (np.is_some() != last_had_media) {
                    last_key = key;
                    last_had_media = np.is_some();
                    if state_tx.send(np).is_err() {
                        return;
                    }
                    on_change();
                } else if np.is_some() {
                    // Refresh timeline periodically while playing.
                    if state_tx.send(np).is_err() {
                        return;
                    }
                }

                thread::sleep(Duration::from_millis(800));
            }
        });

        Ok(Self {
            tx: cmd_tx,
            rx: state_rx,
        })
    }

    pub fn poll(&self) -> Option<Option<NowPlaying>> {
        let mut latest = None;
        while let Ok(v) = self.rx.try_recv() {
            latest = Some(v);
        }
        latest
    }

    pub fn send(&self, cmd: MediaCommand) {
        let _ = self.tx.send(Msg::Cmd(cmd));
    }
}

impl Drop for MediaWatcher {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
    }
}

fn exec_cmd(cmd: MediaCommand) {
    let args: Vec<String> = match cmd {
        MediaCommand::PlayPause => vec!["play-pause".into()],
        MediaCommand::Next => vec!["next".into()],
        MediaCommand::Previous => vec!["previous".into()],
        MediaCommand::Seek(secs) => vec!["position".into(), format!("{:.2}", secs.max(0.0))],
    };
    let _ = Command::new("playerctl").args(&args).output();
}

fn poll_playerctl(cached_art: &mut Option<(String, Option<Arc<Art>>)>) -> Option<NowPlaying> {
    let out = Command::new("playerctl")
        .args([
            "metadata",
            "--format",
            "{{playerName}}\t{{status}}\t{{title}}\t{{artist}}\t{{mpris:length}}\t{{position}}\t{{mpris:artUrl}}",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
    if parts.len() < 3 {
        return None;
    }
    let player = parts[0].trim();
    let status = parts[1].trim();
    let title = parts[2].trim();
    if title.is_empty() && status.is_empty() {
        return None;
    }
    let artist = parts.get(3).map(|s| s.trim()).unwrap_or("");
    let len_us = parts.get(4).and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
    let pos_us = parts.get(5).and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
    let art_url = parts.get(6).map(|s| s.trim()).unwrap_or("");

    let timeline = if len_us > 0 {
        Some((pos_us as f64 / 1_000_000.0, len_us as f64 / 1_000_000.0))
    } else {
        None
    };

    let art = if art_url.is_empty() {
        None
    } else if let Some((url, a)) = cached_art
        && url == art_url
    {
        a.clone()
    } else {
        let loaded = load_art(art_url);
        *cached_art = Some((art_url.to_owned(), loaded.clone()));
        loaded
    };

    Some(NowPlaying {
        source: clean_source(player),
        title: title.to_owned(),
        artist: artist.to_owned(),
        playing: status.eq_ignore_ascii_case("Playing"),
        can_previous: true,
        can_next: true,
        can_toggle: true,
        timeline,
        captured: Instant::now(),
        art,
    })
}

fn load_art(url: &str) -> Option<Arc<Art>> {
    let bytes = if let Some(path) = url.strip_prefix("file://") {
        std::fs::read(path).ok()?
    } else if url.starts_with('/') {
        std::fs::read(url).ok()?
    } else {
        return None;
    };
    let img = image::load_from_memory(&bytes).ok()?;
    let resized = img.thumbnail(ART_SIZE, ART_SIZE).to_rgba8();
    let (w, h) = resized.dimensions();
    Some(Arc::new(Art {
        width: w as usize,
        height: h as usize,
        rgba: resized.into_raw(),
    }))
}

pub fn clean_source(id: &str) -> String {
    let s = id.split('.').next_back().unwrap_or(id);
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => s.to_owned(),
    }
}
