//! Dados simulados para rodar fora do Windows.

use super::{Battery, MediaInfo, Network};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn init_thread() {}

pub struct Media {
    start: i64,
}

impl Media {
    pub fn new() -> Self {
        Self { start: now_ms() }
    }
    pub fn snapshot(&mut self) -> Option<MediaInfo> {
        let elapsed = (now_ms() - self.start) as f64 / 1000.0;
        Some(MediaInfo {
            title: "Midnight City".into(),
            artist: "M83".into(),
            app: "Spotify.exe".into(),
            playing: true,
            position: elapsed % 243.0,
            duration: 243.0,
            updated_ms: now_ms(),
        })
    }
    pub fn artwork(&self) -> Option<String> {
        None
    }
}

pub fn media_command(_cmd: &str) {}

pub struct Fast;

impl Fast {
    pub fn new() -> Self {
        Fast
    }
    pub fn volume(&mut self) -> Option<(f32, bool)> {
        Some((0.5, false))
    }
    pub fn caps_lock(&self) -> bool {
        false
    }
    pub fn clipboard_seq(&self) -> u32 {
        0
    }
    pub fn fullscreen(&self) -> bool {
        false
    }
}

pub fn battery() -> Option<Battery> {
    Some(Battery { percent: 78, charging: false })
}

pub fn network() -> Network {
    Network { online: true, wifi: true, name: "Casa_5G".into() }
}

pub fn clipboard_seq() -> u32 {
    0
}
