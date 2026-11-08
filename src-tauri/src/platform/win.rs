//! Implementação Windows: SMTC (mídia), Core Audio (volume), energia,
//! conectividade, Caps Lock, clipboard e detecção de tela cheia.

use super::{Battery, MediaInfo, Network};
use base64::Engine;
use std::time::{Duration, Instant};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
use windows::Storage::Streams::DataReader;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};

/// Inicializa COM/WinRT na thread atual (MTA). Chamar uma vez por thread.
pub fn init_thread() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// Ticks de 100 ns desde 1601 -> epoch em ms
fn filetime_to_unix_ms(ticks: i64) -> i64 {
    (ticks - 116_444_736_000_000_000) / 10_000
}

// ---------------------------------------------------------------- Mídia

pub struct Media {
    mgr: Option<Manager>,
    last_props: Option<Props>,
    last_try: Instant,
}

impl Media {
    pub fn new() -> Self {
        init_thread();
        Self { mgr: Self::manager(), last_props: None, last_try: Instant::now() }
    }

    fn manager() -> Option<Manager> {
        Manager::RequestAsync().ok()?.join().ok()
    }

    fn session(&mut self) -> Option<Session> {
        if self.mgr.is_none() && self.last_try.elapsed() > Duration::from_secs(5) {
            self.last_try = Instant::now();
            self.mgr = Self::manager();
        }
        self.mgr.as_ref()?.GetCurrentSession().ok()
    }

    pub fn snapshot(&mut self) -> Option<MediaInfo> {
        let s = self.session()?;
        let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let title = props.Title().ok()?.to_string();
        if title.is_empty() {
            return None;
        }
        let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
        let app = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
        let playing = s
            .GetPlaybackInfo()
            .and_then(|p| p.PlaybackStatus())
            .map(|st| st == Status::Playing)
            .unwrap_or(false);

        let (mut position, mut duration, mut updated_ms) = (0.0, 0.0, 0);
        if let Ok(tl) = s.GetTimelineProperties() {
            let start = tl.StartTime().map(|t| t.Duration).unwrap_or(0);
            let end = tl.EndTime().map(|t| t.Duration).unwrap_or(0);
            position = tl.Position().map(|t| t.Duration).unwrap_or(0) as f64 / 1e7;
            duration = (end - start).max(0) as f64 / 1e7;
            updated_ms = tl
                .LastUpdatedTime()
                .map(|t| filetime_to_unix_ms(t.UniversalTime))
                .unwrap_or(0);
        }

        self.last_props = Some(props);
        Some(MediaInfo { title, artist, app, playing, position, duration, updated_ms })
    }

    /// Capa da faixa atual como data URL (chamar quando a faixa muda).
    pub fn artwork(&self) -> Option<String> {
        let props = self.last_props.as_ref()?;
        let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
        let size = stream.Size().ok()? as u32;
        if size == 0 || size > 8_000_000 {
            return None;
        }
        let ctype = stream.ContentType().map(|h| h.to_string()).unwrap_or_default();
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size).ok()?.join().ok()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf).ok()?;
        let mime = if ctype.starts_with("image/") { ctype } else { "image/png".into() };
        Some(format!(
            "data:{};base64,{}",
            mime,
            base64::engine::general_purpose::STANDARD.encode(buf)
        ))
    }
}

pub fn media_command(cmd: &str) {
    init_thread();
    let Some(mgr) = Manager::RequestAsync().ok().and_then(|o| o.join().ok()) else { return };
    let Ok(s) = mgr.GetCurrentSession() else { return };
    let op = match cmd {
        "toggle" => s.TryTogglePlayPauseAsync(),
        "next" => s.TrySkipNextAsync(),
        "prev" => s.TrySkipPreviousAsync(),
        _ => return,
    };
    if let Ok(op) = op {
        let _ = op.join();
    }
}

// ------------------------------------------------- Leituras rápidas (~30 Hz)

pub struct Fast {
    ep: Option<IAudioEndpointVolume>,
    ep_at: Instant,
}

impl Fast {
    pub fn new() -> Self {
        init_thread();
        Self { ep: Self::endpoint(), ep_at: Instant::now() }
    }

    fn endpoint() -> Option<IAudioEndpointVolume> {
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
        }
    }

    /// (volume 0..1, mudo)
    pub fn volume(&mut self) -> Option<(f32, bool)> {
        // o dispositivo padrão pode mudar (fone, HDMI...)
        if self.ep_at.elapsed() > Duration::from_secs(3) {
            self.ep = Self::endpoint();
            self.ep_at = Instant::now();
        }
        let ep = self.ep.as_ref()?;
        unsafe {
            let v = ep.GetMasterVolumeLevelScalar().ok()?;
            let m = ep.GetMute().ok()?.as_bool();
            Some((v, m))
        }
    }

    pub fn caps_lock(&self) -> bool {
        unsafe { GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 }
    }

    pub fn clipboard_seq(&self) -> u32 {
        clipboard_seq()
    }

    /// Jogo, vídeo em tela cheia ou apresentação em primeiro plano
    pub fn fullscreen(&self) -> bool {
        unsafe {
            match SHQueryUserNotificationState() {
                Ok(s) => s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE,
                Err(_) => false,
            }
        }
    }
}

// ------------------------------------------------- Leituras lentas (~1 Hz)

pub fn battery() -> Option<Battery> {
    let mut st = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut st).ok()? };
    if st.BatteryFlag == 128 || st.BatteryLifePercent == 255 {
        return None; // desktop sem bateria
    }
    Some(Battery { percent: st.BatteryLifePercent.min(100), charging: st.ACLineStatus == 1 })
}

pub fn network() -> Network {
    let Ok(p) = NetworkInformation::GetInternetConnectionProfile() else {
        return Network::default();
    };
    let online = p
        .GetNetworkConnectivityLevel()
        .map(|l| l == NetworkConnectivityLevel::InternetAccess)
        .unwrap_or(false);
    let wifi = p.IsWlanConnectionProfile().unwrap_or(false);
    let name = p.ProfileName().map(|h| h.to_string()).unwrap_or_default();
    Network { online, wifi, name }
}

pub fn clipboard_seq() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}
