//! Implementação Windows: SMTC (mídia), Core Audio (volume), energia,
//! conectividade, Caps Lock, clipboard e detecção de tela cheia.

use super::{Battery, MediaInfo, Network, NotifInfo, PowerInfo, PowerPlan, WifiNet};
use base64::Engine;
use std::time::{Duration, Instant};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamWithContentType};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::core::BSTR;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_ALL, COINIT_MULTITHREADED, EOAC_NONE,
    RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Variant::{VariantClear, VARIANT, VT_I4, VT_UI1};
use windows::Win32::System::Wmi::{
    IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
};
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
        read_image(&stream)
    }
}

/// Lê um stream de imagem inteiro e devolve como data URL.
/// Em blocos: alguns players (navegadores) informam tamanho 0.
fn read_image(stream: &IRandomAccessStreamWithContentType) -> Option<String> {
    let ctype = stream.ContentType().map(|h| h.to_string()).unwrap_or_default();
    let reader = DataReader::CreateDataReader(stream).ok()?;
    let mut out: Vec<u8> = Vec::new();
    loop {
        let n = reader.LoadAsync(64 * 1024).ok()?.join().ok()?;
        if n == 0 {
            break;
        }
        let mut buf = vec![0u8; n as usize];
        reader.ReadBytes(&mut buf).ok()?;
        out.extend_from_slice(&buf);
        if out.len() > 8_000_000 {
            return None;
        }
    }
    if out.len() < 64 {
        return None;
    }
    let mime = if ctype.starts_with("image/") {
        ctype
    } else if out.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg".into()
    } else {
        "image/png".into()
    };
    Some(format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(out)))
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

    /// (algo em tela cheia, tela cheia exclusiva). Exclusiva = jogo D3D ou
    /// apresentação: aí nada sobrepõe e a janela some de verdade. O resto
    /// (vídeo, app sem borda em tela cheia) deixa a ilha "puxável".
    pub fn fullscreen(&self) -> (bool, bool) {
        unsafe {
            match SHQueryUserNotificationState() {
                Ok(s) => {
                    let exclusive = s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE;
                    (exclusive || s == QUNS_BUSY, exclusive)
                }
                Err(_) => (false, false),
            }
        }
    }

    /// A janela em primeiro plano passa por baixo da ilha e ocupa a maior
    /// parte do monitor (maximizada ou ≥ 60%)? Ex.: navegador maximizado,
    /// com as abas sob a pílula. `zone` = área da ilha em px físicos
    /// (esq, topo, dir, base); `own` = HWND da própria ilha.
    /// `None` quando a ilha está em primeiro plano (clicaram nela): mantém o estado.
    pub fn covering(&self, own: isize, zone: (i32, i32, i32, i32)) -> Option<bool> {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetWindowRect, IsIconic, IsWindowVisible, IsZoomed,
        };
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return Some(false);
            }
            if fg.0 as isize == own {
                return None;
            }
            if !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() {
                return Some(false);
            }
            // área de trabalho e barra de tarefas não contam
            let mut cls = [0u16; 64];
            let n = GetClassNameW(fg, &mut cls) as usize;
            let cls = String::from_utf16_lossy(&cls[..n]);
            if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
                return Some(false);
            }
            let mut r = RECT::default();
            if GetWindowRect(fg, &mut r).is_err() {
                return Some(false);
            }
            let (zl, zt, zr, zb) = zone;
            let under_island = r.left < zr && r.right > zl && r.top < zb && r.bottom > zt;
            if !under_island {
                return Some(false);
            }
            if IsZoomed(fg).as_bool() {
                return Some(true);
            }
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if !GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
                return Some(false);
            }
            let m = mi.rcMonitor;
            let ix = (r.right.min(m.right) - r.left.max(m.left)).max(0) as i64;
            let iy = (r.bottom.min(m.bottom) - r.top.max(m.top)).max(0) as i64;
            let area = ((m.right - m.left) as i64) * ((m.bottom - m.top) as i64);
            Some(area > 0 && ix * iy * 10 >= area * 6)
        }
    }
}

// ----------------------------------------------------- Brilho da tela (WMI)

/// Brilho da tela interna via WMI (`root\WMI` → `WmiMonitorBrightness`).
/// A conexão abre uma vez; a consulta roda a cada leitura porque o valor muda.
/// Precisa de COM iniciado na thread (`init_thread`) e deve ficar nela.
pub struct Brightness {
    svc: IWbemServices,
}

impl Brightness {
    pub fn new() -> Option<Self> {
        unsafe {
            let loc: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_ALL).ok()?;
            let svc = loc
                .ConnectServer(&BSTR::from("ROOT\\WMI"), &BSTR::new(), &BSTR::new(), &BSTR::new(), 0, &BSTR::new(), None)
                .ok()?;
            // 10 = RPC_C_AUTHN_WINNT, 0 = RPC_C_AUTHZ_NONE
            CoSetProxyBlanket(
                &svc,
                10,
                0,
                windows::core::PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
            .ok()?;
            Some(Self { svc })
        }
    }

    /// Brilho 0..100. `None` sem tela interna (monitor externo, desktop) ou em caso de erro.
    pub fn read(&self) -> Option<u8> {
        unsafe {
            let en = self
                .svc
                .ExecQuery(
                    &BSTR::from("WQL"),
                    &BSTR::from("SELECT CurrentBrightness FROM WmiMonitorBrightness"),
                    WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                    None,
                )
                .ok()?;
            let mut objs = [None];
            let mut got = 0u32;
            let _ = en.Next(WBEM_INFINITE, &mut objs, &mut got);
            if got == 0 {
                return None;
            }
            let obj = objs[0].take()?;
            let mut v = VARIANT::default();
            obj.Get(windows::core::w!("CurrentBrightness"), 0, &mut v, None, None).ok()?;
            let vt = v.Anonymous.Anonymous.vt;
            let out = if vt == VT_UI1 {
                Some(v.Anonymous.Anonymous.Anonymous.bVal)
            } else if vt == VT_I4 {
                Some(v.Anonymous.Anonymous.Anonymous.lVal.clamp(0, 100) as u8)
            } else {
                None
            };
            let _ = VariantClear(&mut v);
            out.map(|b| b.min(100))
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

// ------------------------------------------------------------------ Wi-Fi

use std::ptr::null_mut;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::NetworkManagement::WiFi::{
    dot11_BSS_type_infrastructure, wlan_connection_mode_profile, WlanCloseHandle, WlanConnect,
    WlanEnumInterfaces, WlanFreeMemory, WlanGetAvailableNetworkList, WlanOpenHandle, WlanScan,
    WLAN_AVAILABLE_NETWORK_CONNECTED, WLAN_AVAILABLE_NETWORK_HAS_PROFILE, WLAN_AVAILABLE_NETWORK_LIST,
    WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST,
};

struct Wlan(HANDLE);

impl Wlan {
    fn open() -> Result<Self, String> {
        let mut ver = 0u32;
        let mut h = HANDLE::default();
        let r = unsafe { WlanOpenHandle(2, None, &mut ver, &mut h) };
        if r != 0 {
            return Err(format!("wlan:{r}"));
        }
        Ok(Wlan(h))
    }

    fn first_iface(&self) -> Option<GUID> {
        unsafe {
            let mut list: *mut WLAN_INTERFACE_INFO_LIST = null_mut();
            if WlanEnumInterfaces(self.0, None, &mut list) != 0 || list.is_null() {
                return None;
            }
            let l = &*list;
            let g = if l.dwNumberOfItems > 0 { Some(l.InterfaceInfo[0].InterfaceGuid) } else { None };
            WlanFreeMemory(list as *const _);
            g
        }
    }
}

impl Drop for Wlan {
    fn drop(&mut self) {
        unsafe {
            WlanCloseHandle(self.0, None);
        }
    }
}

fn wide_to_string(w: &[u16]) -> String {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

/// Redes visíveis. Erro "location" = Windows exige permissão de Localização
/// para apps de desktop lerem nomes de redes (Windows 11 24H2+).
pub fn wifi_list() -> Result<Vec<WifiNet>, String> {
    let w = Wlan::open()?;
    let Some(guid) = w.first_iface() else { return Err("noadapter".into()) };
    unsafe {
        // dispara uma varredura nova; o resultado chega em ~2-4 s
        let _ = WlanScan(w.0, &guid, None, None, None);
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = null_mut();
        let r = WlanGetAvailableNetworkList(w.0, &guid, 0, None, &mut list);
        if r == 5 {
            return Err("location".into());
        }
        if r != 0 || list.is_null() {
            return Err(format!("wlan:{r}"));
        }
        let l = &*list;
        let items = std::slice::from_raw_parts(l.Network.as_ptr(), l.dwNumberOfItems as usize);
        let mut out: Vec<WifiNet> = Vec::new();
        for n in items {
            let len = (n.dot11Ssid.uSSIDLength as usize).min(32);
            let ssid = String::from_utf8_lossy(&n.dot11Ssid.ucSSID[..len]).to_string();
            if ssid.is_empty() {
                continue; // rede oculta
            }
            let saved = n.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0;
            let connected = n.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0;
            let profile = wide_to_string(&n.strProfileName);
            let item = WifiNet {
                ssid: ssid.clone(),
                signal: n.wlanSignalQuality,
                secure: n.bSecurityEnabled.as_bool(),
                saved,
                connected,
                profile: profile.clone(),
            };
            // o Windows repete a mesma rede (com e sem perfil): mescla
            if let Some(e) = out.iter_mut().find(|e| e.ssid == ssid) {
                e.signal = e.signal.max(item.signal);
                e.connected |= connected;
                if saved {
                    e.saved = true;
                    e.profile = profile;
                }
            } else {
                out.push(item);
            }
        }
        WlanFreeMemory(list as *const _);
        out.sort_by(|a, b| b.connected.cmp(&a.connected).then(b.signal.cmp(&a.signal)));
        Ok(out)
    }
}

/// Conecta numa rede que já tem perfil salvo no Windows.
pub fn wifi_connect(profile: &str) -> Result<(), String> {
    let w = Wlan::open()?;
    let Some(guid) = w.first_iface() else { return Err("noadapter".into()) };
    let wide: Vec<u16> = profile.encode_utf16().chain(std::iter::once(0)).collect();
    let params = WLAN_CONNECTION_PARAMETERS {
        wlanConnectionMode: wlan_connection_mode_profile,
        strProfile: PCWSTR(wide.as_ptr()),
        pDot11Ssid: null_mut(),
        pDesiredBssidList: null_mut(),
        dot11BssType: dot11_BSS_type_infrastructure,
        dwFlags: 0,
    };
    let r = unsafe { WlanConnect(w.0, &guid, &params, None) };
    if r == 0 { Ok(()) } else { Err(format!("wlan:{r}")) }
}

// ------------------------------------------------------------------ Energia

use windows::core::{s, w};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Power::{
    PowerEnumerate, PowerGetActiveScheme, PowerReadFriendlyName, PowerSetActiveScheme, ACCESS_SCHEME,
};

// Modos de energia do Windows 11 (overlay sobre o plano Equilibrado)
const MODE_EFFICIENCY: u128 = 0x961cc777_2547_4f9d_8174_7d86181b8a7a;
const MODE_BETTER_BATTERY: u128 = 0x3af9b8d9_7c97_431d_ad78_34a8bfea439f;
const MODE_BALANCED: u128 = 0;
const MODE_PERFORMANCE: u128 = 0xded574b5_45a0_4f42_8737_46345c09c238;

fn guid_str(g: &GUID) -> String {
    format!("{g:?}")
}

fn parse_guid(s: &str) -> Option<GUID> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok().map(GUID::from_u128)
}

fn friendly_name(g: &GUID) -> String {
    unsafe {
        let mut size = 0u32;
        let _ = PowerReadFriendlyName(None, Some(g), None, None, None, &mut size);
        if size == 0 {
            return String::new();
        }
        let mut buf = vec![0u8; size as usize];
        if PowerReadFriendlyName(None, Some(g), None, None, Some(buf.as_mut_ptr()), &mut size).0 != 0 {
            return String::new();
        }
        let w: Vec<u16> = buf.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        wide_to_string(&w)
    }
}

fn active_scheme() -> Option<GUID> {
    unsafe {
        let mut p: *mut GUID = null_mut();
        if PowerGetActiveScheme(None, &mut p).0 != 0 || p.is_null() {
            return None;
        }
        let g = *p;
        let _ = LocalFree(Some(HLOCAL(p as *mut core::ffi::c_void)));
        Some(g)
    }
}

type FnGetOverlay = unsafe extern "system" fn(*mut GUID) -> u32;
type FnSetOverlay = unsafe extern "system" fn(GUID) -> u32;

fn overlay_get() -> Option<GUID> {
    unsafe {
        let h = LoadLibraryW(w!("powrprof.dll")).ok()?;
        let f = GetProcAddress(h, s!("PowerGetEffectiveOverlayScheme"))?;
        let f: FnGetOverlay = std::mem::transmute(f);
        let mut g = GUID::zeroed();
        if f(&mut g) == 0 { Some(g) } else { None }
    }
}

fn overlay_set(g: GUID) -> Result<(), String> {
    unsafe {
        let h = LoadLibraryW(w!("powrprof.dll")).map_err(|e| e.to_string())?;
        let f = GetProcAddress(h, s!("PowerSetActiveOverlayScheme")).ok_or("sem suporte")?;
        let f: FnSetOverlay = std::mem::transmute(f);
        let r = f(g);
        if r == 0 { Ok(()) } else { Err(format!("power:{r}")) }
    }
}

pub fn power_info() -> PowerInfo {
    let active = active_scheme();
    let mut plans = Vec::new();
    for i in 0..32u32 {
        let mut g = GUID::zeroed();
        let mut size = std::mem::size_of::<GUID>() as u32;
        let r = unsafe {
            PowerEnumerate(None, None, None, ACCESS_SCHEME, i, Some(&mut g as *mut GUID as *mut u8), &mut size)
        };
        if r.0 != 0 {
            break;
        }
        plans.push(PowerPlan { guid: guid_str(&g), name: friendly_name(&g), active: Some(g) == active });
    }

    let mode = overlay_get().map(|g| {
        match g.to_u128() {
            MODE_EFFICIENCY | MODE_BETTER_BATTERY => "efficiency",
            MODE_PERFORMANCE => "performance",
            _ => "balanced",
        }
        .to_string()
    });

    let mut st = SYSTEM_POWER_STATUS::default();
    let ok = unsafe { GetSystemPowerStatus(&mut st).is_ok() };
    let has_batt = ok && st.BatteryFlag != 128 && st.BatteryLifePercent != 255;
    let ac = ok && st.ACLineStatus == 1;
    PowerInfo {
        plans,
        mode,
        saver: ok && st.SystemStatusFlag == 1,
        ac,
        percent: if has_batt { Some(st.BatteryLifePercent.min(100)) } else { None },
        time_left: if has_batt && !ac && st.BatteryLifeTime != u32::MAX { Some(st.BatteryLifeTime) } else { None },
    }
}

pub fn set_power_plan(guid: &str) -> Result<(), String> {
    let g = parse_guid(guid).ok_or("guid inválido")?;
    let r = unsafe { PowerSetActiveScheme(None, Some(&g)) };
    if r.0 == 0 { Ok(()) } else { Err(format!("power:{}", r.0)) }
}

pub fn set_power_mode(mode: &str) -> Result<(), String> {
    let g = match mode {
        "efficiency" => MODE_EFFICIENCY,
        "performance" => MODE_PERFORMANCE,
        _ => MODE_BALANCED,
    };
    overlay_set(GUID::from_u128(g))
}

// ------------------------------------------------------------ Notificações

use std::collections::HashSet;
use windows::ApplicationModel::Package;
use windows::Foundation::Size;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus as Access,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};

/// O processo foi iniciado com identidade de pacote (pacote esparso registrado)?
pub fn has_identity() -> bool {
    Package::Current().is_ok()
}

/// "allowed" | "denied" | "unspecified" | "noidentity" | "error"
pub fn notif_access(request: bool) -> String {
    if !has_identity() {
        return "noidentity".into();
    }
    let Ok(l) = UserNotificationListener::Current() else { return "error".into() };
    let st = if request {
        l.RequestAccessAsync().and_then(|o| o.join()).or_else(|_| l.GetAccessStatus())
    } else {
        l.GetAccessStatus()
    };
    match st {
        Ok(Access::Allowed) => "allowed",
        Ok(Access::Denied) => "denied",
        Ok(_) => "unspecified",
        Err(_) => "error",
    }
    .into()
}

pub struct Notifs {
    l: UserNotificationListener,
    seen: HashSet<u32>,
    first: bool,
}

impl Notifs {
    pub fn new() -> Option<Self> {
        init_thread();
        Some(Self { l: UserNotificationListener::Current().ok()?, seen: HashSet::new(), first: true })
    }

    /// Notificações novas desde a última chamada (a primeira só marca as que já existiam).
    pub fn poll(&mut self) -> Vec<NotifInfo> {
        let Ok(list) = self.l.GetNotificationsAsync(NotificationKinds::Toast).and_then(|o| o.join()) else {
            return Vec::new();
        };
        let n = list.Size().unwrap_or(0);
        let mut now: HashSet<u32> = HashSet::new();
        let mut out = Vec::new();
        for i in 0..n {
            let Ok(un) = list.GetAt(i) else { continue };
            let Ok(id) = un.Id() else { continue };
            now.insert(id);
            if self.first || self.seen.contains(&id) {
                continue;
            }
            if let Some(info) = describe(&un, id) {
                out.push(info);
            }
        }
        self.first = false;
        self.seen = now;
        out
    }
}

fn describe(un: &UserNotification, id: u32) -> Option<NotifInfo> {
    let display = un.AppInfo().and_then(|a| a.DisplayInfo()).ok();
    let app = display
        .as_ref()
        .and_then(|d| d.DisplayName().ok())
        .map(|h| h.to_string())
        .unwrap_or_default();
    let icon = display.as_ref().and_then(|d| {
        let r = d.GetLogo(Size { Width: 64.0, Height: 64.0 }).ok()?;
        let s = r.OpenReadAsync().ok()?.join().ok()?;
        read_image(&s)
    });
    let texts: Vec<String> = (|| -> windows::core::Result<Vec<String>> {
        let binding = un.Notification()?.Visual()?.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
        let els = binding.GetTextElements()?;
        let mut v = Vec::new();
        for i in 0..els.Size()? {
            v.push(els.GetAt(i)?.Text()?.to_string());
        }
        Ok(v)
    })()
    .unwrap_or_default();
    if app.is_empty() && texts.is_empty() {
        return None;
    }
    let time_ms = un.CreationTime().map(|t| filetime_to_unix_ms(t.UniversalTime)).unwrap_or(0);
    Some(NotifInfo {
        id,
        app,
        title: texts.first().cloned().unwrap_or_default(),
        body: texts.iter().skip(1).cloned().collect::<Vec<_>>().join(" · "),
        icon,
        time_ms,
    })
}

// ------------------------------------------------------------- usuário e energia
use super::UserInfo;

/// Nome completo (Configurações > Contas) e foto da conta.
pub fn user_info() -> UserInfo {
    let login = std::env::var("USERNAME").unwrap_or_default();
    let name = user::full_name(&login).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| login.clone());
    UserInfo { name, login, picture: user::account_picture() }
}

/// "sleep" | "restart" | "shutdown"
pub fn power_action(action: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let args: &[&str] = match action {
        "restart" => &["/r", "/t", "0"],
        "shutdown" => &["/s", "/t", "0"],
        "sleep" => {
            // suspender (não hibernar): SetSuspendState(hibernate = false)
            let ok = unsafe { windows::Win32::System::Power::SetSuspendState(false, false, false) };
            return if ok { Ok(()) } else { Err("o Windows recusou a suspensão".into()) };
        }
        _ => return Err(format!("ação desconhecida: {action}")),
    };
    std::process::Command::new("shutdown")
        .args(args)
        .creation_flags(0x0800_0000) // sem janela de console
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

mod user {
    use base64::Engine;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
    use windows::Win32::NetworkManagement::NetManagement::{NetApiBufferFree, NetUserGetInfo, USER_INFO_2};
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// "Nome completo" da conta local (para conta Microsoft, o nome dela).
    pub fn full_name(login: &str) -> Option<String> {
        let wl = wide(login);
        let mut buf: *mut u8 = std::ptr::null_mut();
        let rc = unsafe { NetUserGetInfo(PCWSTR::null(), PCWSTR(wl.as_ptr()), 2, &mut buf) };
        if rc != 0 || buf.is_null() {
            return None;
        }
        let name = unsafe { (*(buf as *const USER_INFO_2)).usri2_full_name.to_string().ok() };
        unsafe { NetApiBufferFree(Some(buf as _)) };
        name
    }

    fn current_sid() -> Option<String> {
        unsafe {
            let mut tok = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok).ok()?;
            let mut len = 0u32;
            let _ = GetTokenInformation(tok, TokenUser, None, 0, &mut len);
            let mut buf = vec![0u8; len as usize];
            let r = GetTokenInformation(tok, TokenUser, Some(buf.as_mut_ptr() as _), len, &mut len);
            let _ = CloseHandle(tok);
            r.ok()?;
            let tu = &*(buf.as_ptr() as *const TOKEN_USER);
            let mut s = PWSTR::null();
            ConvertSidToStringSidW(tu.User.Sid, &mut s).ok()?;
            let out = s.to_string().ok();
            let _ = LocalFree(Some(HLOCAL(s.0 as _)));
            out
        }
    }

    fn reg_hklm_string(key: &str, value: &str) -> Option<String> {
        let (k, v) = (wide(key), wide(value));
        let mut buf = vec![0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let rc = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(k.as_ptr()),
                PCWSTR(v.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as _),
                Some(&mut size),
            )
        };
        if rc.is_err() {
            return None;
        }
        let n = (size as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
    }

    fn data_url(b: &[u8]) -> String {
        let mime = if b.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else if b.starts_with(b"BM") { "image/bmp" } else { "image/jpeg" };
        format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(b))
    }

    /// JPEGs embutidos num .accountpicture-ms (vêm em 96 px e 448 px).
    fn embedded_jpegs(b: &[u8]) -> Vec<&[u8]> {
        let starts: Vec<usize> = b.windows(3).enumerate().filter(|(_, w)| *w == [0xFF, 0xD8, 0xFF]).map(|(i, _)| i).collect();
        let mut out = Vec::new();
        for (i, &s) in starts.iter().enumerate() {
            let end_lim = starts.get(i + 1).copied().unwrap_or(b.len());
            // último FFD9 antes do próximo início = fim deste JPEG
            if let Some(e) = b[s..end_lim].windows(2).rposition(|w| w == [0xFF, 0xD9]) {
                out.push(&b[s..s + e + 2]);
            }
        }
        out
    }

    /// Foto da conta: 1) caminho registrado pela tela de Contas;
    /// 2) cache da conta Microsoft em %APPDATA%. Sem foto: None (a UI usa a inicial).
    pub fn account_picture() -> Option<String> {
        if let Some(sid) = current_sid() {
            let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AccountPicture\Users\{sid}");
            for v in ["Image448", "Image240", "Image192", "Image96"] {
                if let Some(bytes) = reg_hklm_string(&key, v).and_then(|p| std::fs::read(p).ok()) {
                    return Some(data_url(&bytes));
                }
            }
        }
        let dir = std::path::PathBuf::from(std::env::var("APPDATA").ok()?).join(r"Microsoft\Windows\AccountPictures");
        let mut best: Option<Vec<u8>> = None;
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("accountpicture-ms")) {
                continue;
            }
            if let Ok(b) = std::fs::read(&p) {
                for img in embedded_jpegs(&b) {
                    if best.as_ref().is_none_or(|x| img.len() > x.len()) {
                        best = Some(img.to_vec());
                    }
                }
            }
        }
        best.map(|b| data_url(&b))
    }
}
