//! Dados simulados para rodar fora do Windows.

use super::{Battery, MediaInfo, Network, NotifInfo, PowerInfo, PowerPlan, WifiNet};
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

pub fn media_seek(_secs: f64) {}

/// Aviso de volume do sistema: não existe fora do Windows.
pub struct VolumeOsd;

impl VolumeOsd {
    pub fn new() -> Self {
        VolumeOsd
    }
    pub fn visible(&mut self) -> bool {
        false
    }
    pub fn hide(&self) {}
}

#[derive(Clone)]
pub struct Volume;

impl Volume {
    pub fn start(_on_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
        Volume
    }
    pub fn get(&self) -> Option<(f32, bool)> {
        Some((0.5, false))
    }
}

pub struct Fast;

impl Fast {
    pub fn new() -> Self {
        Fast
    }
    pub fn caps_lock(&self) -> bool {
        false
    }
    pub fn clipboard_seq(&self) -> u32 {
        0
    }
    pub fn fullscreen(&self) -> (bool, bool) {
        (false, false)
    }

    pub fn covering(&self, _own: isize, _zone: (i32, i32, i32, i32)) -> Option<bool> {
        Some(false)
    }
}

/// Sem tela interna no mock: nunca devolve brilho.
pub struct Brightness;

impl Brightness {
    pub fn new() -> Option<Self> {
        Some(Brightness)
    }
    pub fn read(&self) -> Option<u8> {
        None
    }
    pub fn watch(&self) -> Option<BrightnessWatch> {
        None
    }
}

pub struct BrightnessWatch;

impl BrightnessWatch {
    pub fn next(&mut self) -> Option<u8> {
        None
    }
}

/// Cor de destaque fixa: o azul padrão do Windows, que nunca muda no mock.
pub struct Accent;

impl Accent {
    pub fn new() -> Option<Self> {
        Some(Accent)
    }
    pub fn read(&self) -> Option<String> {
        Some("#0078d4".into())
    }
    pub fn watch(&self) -> Option<AccentWatch> {
        None
    }
}

pub struct AccentWatch;

impl AccentWatch {
    pub fn next(&self) -> Option<String> {
        None
    }
}

/// Nenhum USB chega no mock: `start` falha e a thread só tenta de novo.
pub struct UsbWatch;

impl UsbWatch {
    pub fn start() -> Option<Self> {
        None
    }
    pub fn next(&self) -> Option<String> {
        None
    }
}

/// Sem captura fora do Windows: a onda sai só com o brilho, sem distorção.
pub fn capture_screen(_x: i32, _y: i32, _w: i32, _h: i32) -> Option<Vec<u8>> {
    None
}

pub fn show_below(_win: isize, _above: isize) {}

pub fn hide_window(_win: isize) {}

pub fn battery() -> Option<Battery> {
    Some(Battery { percent: 78, charging: false })
}

pub fn network() -> Network {
    Network { online: true, wifi: true, name: "Casa_5G".into() }
}

pub fn clipboard_seq() -> u32 {
    0
}

pub fn wifi_list() -> Result<Vec<WifiNet>, String> {
    let n = |ssid: &str, signal, secure, saved, connected| WifiNet {
        ssid: ssid.into(), signal, secure, saved, connected, profile: ssid.into(),
    };
    Ok(vec![
        n("Casa_5G", 92, true, true, true),
        n("Casa_2.4G", 80, true, true, false),
        n("VIVO-8A21", 54, true, false, false),
        n("Cafe Livre", 31, false, false, false),
    ])
}

pub fn wifi_connect(_profile: &str) -> Result<(), String> {
    Ok(())
}

pub fn power_info() -> PowerInfo {
    PowerInfo {
        plans: vec![
            PowerPlan { guid: "381B4222-F694-41F0-9685-FF5BB260DF2E".into(), name: "Equilibrado".into(), active: true },
            PowerPlan { guid: "8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C".into(), name: "Alto desempenho".into(), active: false },
        ],
        mode: Some("balanced".into()),
        saver: false,
        ac: false,
        percent: Some(78),
        time_left: Some(3 * 3600 + 25 * 60),
    }
}

pub fn set_power_plan(_guid: &str) -> Result<(), String> {
    Ok(())
}

pub fn set_power_mode(_mode: &str) -> Result<(), String> {
    Ok(())
}

#[allow(dead_code)]
pub fn has_identity() -> bool {
    false
}

pub fn notif_access(_request: bool) -> String {
    "noidentity".into()
}

pub struct Notifs;

impl Notifs {
    pub fn new() -> Option<Self> {
        None
    }
    pub fn poll(&mut self) -> Vec<NotifInfo> {
        Vec::new()
    }
}

pub fn user_info() -> super::UserInfo {
    let login = std::env::var("USER").unwrap_or_else(|_| "rangel".into());
    super::UserInfo { name: "Rangel Macedo".into(), login, picture: None }
}

pub fn power_action(_action: &str) -> Result<(), String> {
    Ok(())
}

pub fn autostart_quote_path() {}
