//! Audio recording & WAV playback on Linux (`arecord`/`parec` + `paplay`/`aplay`).

use crate::{PlatformError, Result};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub const SAMPLE_RATE: u32 = 44_100;

static PLAYER: Mutex<Option<Child>> = Mutex::new(None);

pub fn record(stop: &AtomicBool, mut on_chunk: impl FnMut(&[i16])) -> Result<()> {
    let mut child = Command::new("arecord")
        .args(["-q", "-f", "S16_LE", "-r", "44100", "-c", "1", "-t", "raw"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| PlatformError::Other(format!("arecord: {e}")))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| PlatformError::Other("arecord stdout".into()))?;
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        let Ok(n) = stdout.read(&mut buf) else {
            break;
        };
        if n == 0 {
            break;
        }
        let samples: Vec<i16> = buf[..n - (n % 2)]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        on_chunk(&samples);
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

pub struct WavWriter {
    w: BufWriter<File>,
    data_bytes: u32,
}

impl WavWriter {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let mut w = BufWriter::new(File::create(path)?);
        w.write_all(&[0u8; 44])?;
        Ok(Self { w, data_bytes: 0 })
    }

    pub fn write(&mut self, samples: &[i16]) -> std::io::Result<()> {
        for &s in samples {
            self.w.write_all(&s.to_le_bytes())?;
        }
        self.data_bytes = self.data_bytes.saturating_add((samples.len() * 2) as u32);
        Ok(())
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        self.w.flush()?;
        let mut file = self.w.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(0))?;
        let mut hdr = [0u8; 44];
        hdr[0..4].copy_from_slice(b"RIFF");
        hdr[4..8].copy_from_slice(&(36 + self.data_bytes).to_le_bytes());
        hdr[8..12].copy_from_slice(b"WAVE");
        hdr[12..16].copy_from_slice(b"fmt ");
        hdr[16..20].copy_from_slice(&16u32.to_le_bytes());
        hdr[20..22].copy_from_slice(&1u16.to_le_bytes());
        hdr[22..24].copy_from_slice(&1u16.to_le_bytes());
        hdr[24..28].copy_from_slice(&SAMPLE_RATE.to_le_bytes());
        hdr[28..32].copy_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        hdr[32..34].copy_from_slice(&2u16.to_le_bytes());
        hdr[34..36].copy_from_slice(&16u16.to_le_bytes());
        hdr[36..40].copy_from_slice(b"data");
        hdr[40..44].copy_from_slice(&self.data_bytes.to_le_bytes());
        file.write_all(&hdr)?;
        Ok(())
    }
}

pub fn read_wav(path: &Path) -> std::io::Result<(u32, Vec<i16>)> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 44 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "wav too short",
        ));
    }
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let samples = bytes[44..]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok((rate, samples))
}

pub fn play(path: &Path) -> Result<()> {
    stop_playback();
    let child = Command::new("paplay")
        .arg(path)
        .spawn()
        .or_else(|_| Command::new("aplay").arg("-q").arg(path).spawn())
        .map_err(|e| PlatformError::Other(format!("audio play: {e}")))?;
    if let Ok(mut g) = PLAYER.lock() {
        *g = Some(child);
    }
    Ok(())
}

pub fn play_loop(path: &Path) -> Result<()> {
    play(path)
}

pub fn stop_playback() {
    if let Ok(mut g) = PLAYER.lock()
        && let Some(mut c) = g.take()
    {
        let _ = c.kill();
        let _ = c.wait();
    }
}
