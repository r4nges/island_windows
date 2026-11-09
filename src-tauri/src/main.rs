#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dock;
mod platform;
mod settings;
mod shortcuts;
mod sys;
mod update;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, DragDropEvent, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Área da pílula, em px CSS relativos à janela. Fora dela os cliques
/// atravessam a janela transparente para o app de baixo.
#[derive(Default, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Default)]
struct Shared {
    rect: Mutex<Rect>,
    /// sequência do clipboard gerada por nós (para não virar "Copiado")
    own_clip_seq: AtomicU32,
    /// painel de detalhe aberto no frontend ("cpu", "ram", "gpu", "net")
    detail: Mutex<Option<String>>,
    /// zoom do WebView = escala da ilha (0 enquanto não carregou = 1.0)
    zoom: Mutex<f64>,
    /// monitor escolhido nas configurações (`None` = principal)
    monitor: Mutex<Option<String>>,
    /// posição da ilha no monitor (`settings::anchor`): (na base?, lado -1/0/1);
    /// o padrão (false, 0) é o topo centralizado
    anchor: Mutex<(bool, i32)>,
    /// esconder o aviso de volume do Windows (`hide_volume_osd` com o aviso `volume` ligado)
    hide_osd: AtomicBool,
    /// a página não mostraria o HUD agora (mouse em cima, fixada, arrastando
    /// arquivo, app por baixo): mesmo teste do `hud()` do frontend
    hud_blocked: AtomicBool,
    /// mouse em cima da pílula (o `spawn_fast` atualiza)
    hover: AtomicBool,
    /// app cobrindo a ilha (o `spawn_fast` atualiza)
    covered: AtomicBool,
    /// o callback de volume mudou o volume/mudo: o `spawn_fast` emite
    vol_dirty: AtomicBool,
    /// última mudança de volume que vira HUD na ilha
    osd_at: Mutex<Option<Instant>>,
    /// thread que esconde o aviso do Windows (dorme até `wake_osd`)
    osd_thread: OnceLock<thread::Thread>,
    /// cor de destaque do Windows ("#rrggbb"), para a aura "Sistema" (o `spawn_accent` atualiza)
    accent: Mutex<Option<String>>,
    /// app em tela cheia (jogo, vídeo, apresentação; o `spawn_fast` atualiza): a onda do USB
    /// não toca e a dock da barra de tarefas some
    fullscreen: AtomicBool,
    /// onda do USB na tela (da `play_wave` até a `wave_done`): disparos no meio são ignorados
    wave_on: AtomicBool,
    /// conta as ondas, para o prazo de segurança de uma não esconder a seguinte
    wave_gen: AtomicU32,
    /// captura da tela da onda atual, até a página da onda buscá-la (`wave_frame`)
    wave_frame: Mutex<Option<Vec<u8>>>,
}

impl Shared {
    fn zoom(&self) -> f64 {
        let z = *self.zoom.lock().unwrap();
        if z > 0.0 { z } else { 1.0 }
    }

    /// O volume mudou e a ilha vai mostrar o HUD: acorda a thread do aviso.
    fn wake_osd(&self) {
        *self.osd_at.lock().unwrap() = Some(Instant::now());
        if let Some(t) = self.osd_thread.get() {
            t.unpark();
        }
    }
}

/// Configurações carregadas + caminho do arquivo.
struct SettingsState {
    cur: Mutex<settings::Settings>,
    file: std::path::PathBuf,
}

const BASE_W: f64 = 760.0;
const BASE_H: f64 = 500.0;
/// Distância da pílula à borda lateral da tela nos cantos, em px CSS; igual ao
/// `EDGE` do app.js. A janela encosta na borda e a pílula fica a isso dela.
const SIDE_GAP: f64 = 12.0;

#[derive(Serialize, Clone)]
struct Fast {
    volume: Option<f32>,
    muted: bool,
    caps: bool,
}

#[derive(Serialize, Clone)]
struct Slow {
    stats: sys::Stats,
    battery: Option<platform::Battery>,
    network: platform::Network,
    detail: Option<serde_json::Value>,
}

// ------------------------------------------------------------- comandos

#[tauri::command]
fn set_hit_rect(state: tauri::State<'_, Arc<Shared>>, x: f64, y: f64, w: f64, h: f64) {
    *state.rect.lock().unwrap() = Rect { x, y, w, h };
}

#[tauri::command]
fn media_cmd(cmd: String) {
    thread::spawn(move || platform::media_command(&cmd));
}

#[tauri::command]
fn copy_text(state: tauri::State<'_, Arc<Shared>>, text: String) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())?;
    state.own_clip_seq.store(platform::clipboard_seq(), Ordering::SeqCst);
    Ok(())
}

/// O frontend avisa quando o HUD não apareceria (ver `Shared::hud_blocked`).
#[tauri::command]
fn set_hud_blocked(state: tauri::State<'_, Arc<Shared>>, blocked: bool) {
    state.hud_blocked.store(blocked, Ordering::Relaxed);
}

#[tauri::command]
fn set_detail(state: tauri::State<'_, Arc<Shared>>, kind: Option<String>) {
    *state.detail.lock().unwrap() = kind;
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn wifi_list() -> Result<Vec<platform::WifiNet>, String> {
    blocking(platform::wifi_list).await?
}

#[tauri::command]
async fn wifi_connect(profile: String) -> Result<(), String> {
    blocking(move || platform::wifi_connect(&profile)).await?
}

#[tauri::command]
async fn power_info() -> Result<platform::PowerInfo, String> {
    blocking(platform::power_info).await
}

#[tauri::command]
async fn set_power_plan(guid: String) -> Result<(), String> {
    blocking(move || platform::set_power_plan(&guid)).await?
}

#[tauri::command]
async fn set_power_mode(mode: String) -> Result<(), String> {
    blocking(move || platform::set_power_mode(&mode)).await?
}

/// Abre URIs do sistema (ms-settings:..., ms-availablenetworks:) ou o Gerenciador de Tarefas
#[tauri::command]
fn open_uri(uri: String) {
    let allowed = uri.starts_with("ms-settings:") || uri.starts_with("ms-availablenetworks:") || uri == "taskmgr";
    if !allowed {
        return;
    }
    #[cfg(windows)]
    {
        let _ = if uri == "taskmgr" {
            std::process::Command::new("taskmgr").spawn()
        } else {
            std::process::Command::new("explorer").arg(&uri).spawn()
        };
    }
}

/// Registra o pacote esparso (identidade) e reinicia o app para ela valer.
#[tauri::command]
async fn enable_notifications(app: AppHandle) -> Result<(), String> {
    let dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let manifest = dir.join("AppxManifest.xml");
    if !manifest.exists() {
        return Err(format!("AppxManifest.xml não encontrado em {}", dir.display()));
    }
    let cmd = format!(
        "Add-AppxPackage -Register '{}' -ExternalLocation '{}' -ForceUpdateFromAnyVersion",
        manifest.display(),
        dir.display()
    );
    let out = blocking(move || {
        let mut c = std::process::Command::new("powershell");
        c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &cmd]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // sem janela de console
        }
        c.output()
    })
    .await?
    .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "falha ao registrar o pacote".into() } else { err });
    }
    app.restart();
}

#[tauri::command]
async fn user_info() -> Result<platform::UserInfo, String> {
    blocking(platform::user_info).await
}

/// Painel do usuário: "sleep" | "restart" | "shutdown".
#[tauri::command]
async fn power_action(action: String) -> Result<(), String> {
    blocking(move || platform::power_action(&action)).await?
}

/// Cor de destaque do Windows para a aura "Sistema"; as trocas chegam depois
/// por `island:accent`. Se a página pedir antes da thread ler, lê aqui mesmo.
#[tauri::command]
fn accent_color(shared: tauri::State<'_, Arc<Shared>>) -> Option<String> {
    let cur = shared.accent.lock().unwrap().clone();
    cur.or_else(|| platform::Accent::new()?.read())
}

// ------------------------------------------------------------- onda do USB
// A página principal recebe `island:usb`, mostra o aviso e pede a onda
// (`play_wave`). O Rust captura a tela (só com distorção), cobre o monitor da
// ilha com a janela "wave" (transparente, sem foco, clique atravessa), logo
// abaixo da ilha, e manda `wave:play`. A página da onda busca a captura
// (`wave_frame`), anima e avisa no fim (`wave_done`), e a janela some.

/// Pedido da página principal. Cor = a já resolvida para a aura (`--glow`).
#[derive(Deserialize)]
struct WaveReq {
    color: [u8; 3],
    /// "subtle" | "medium"
    intensity: String,
    /// ms
    duration: u32,
    /// deslocamento máximo do vidro, px CSS (0 = sem distorção, sem captura)
    bulge: f64,
    /// centro da pílula, px lógicos a partir do canto de cima à esquerda da
    /// janela da ilha (o `play_wave` soma onde a janela está no monitor)
    cx: f64,
    cy: f64,
}

/// O que a página da onda recebe: o pedido + o monitor coberto.
#[derive(Serialize, Clone)]
struct WavePlay {
    color: [u8; 3],
    intensity: String,
    duration: u32,
    bulge: f64,
    /// centro da pílula, px lógicos a partir do canto de cima à esquerda do monitor
    cx: f64,
    cy: f64,
    /// tamanho da janela da onda em px físicos (o monitor menos 1 px) e a escala dele
    width: u32,
    height: u32,
    scale: f64,
    /// há captura esperando em `wave_frame`
    capture: bool,
    /// epoch em ms do pedido: a onda sai 140 ms depois dele, com a pílula já abrindo
    at: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Esconde a janela da onda e solta a captura (se a página nem chegou a buscá-la).
fn finish_wave(app: &AppHandle, shared: &Shared) {
    if let Some(Ok(h)) = app.get_webview_window("wave").map(|w| w.hwnd()) {
        platform::hide_window(h.0 as isize);
    }
    *shared.wave_frame.lock().unwrap() = None;
    shared.wave_on.store(false, Ordering::SeqCst);
}

#[tauri::command]
async fn play_wave(app: AppHandle, wave: WaveReq) -> Result<(), String> {
    let shared = app.state::<Arc<Shared>>().inner().clone();
    // nada salta sobre tela cheia (a captura de vídeo protegido sairia preta)
    // e uma onda por vez
    if shared.fullscreen.load(Ordering::Relaxed) || shared.wave_on.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let at = now_ms();
    let (Some(main), Some(wv)) = (app.get_webview_window("main"), app.get_webview_window("wave")) else {
        shared.wave_on.store(false, Ordering::SeqCst);
        return Ok(());
    };
    let Some(m) = target_monitor(&main, &shared) else {
        shared.wave_on.store(false, Ordering::SeqCst);
        return Ok(());
    };
    let (pos, size) = (*m.position(), *m.size());
    // a página manda o centro relativo à própria janela; a onda cobre o monitor
    // e a janela pode estar na base ou num canto dele
    let sf = m.scale_factor();
    let (ox, oy) = main
        .outer_position()
        .map(|p| ((p.x - pos.x) as f64 / sf, (p.y - pos.y) as f64 / sf))
        .unwrap_or((0.0, 0.0));
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let cx = (ox + finite(wave.cx)).clamp(0.0, size.width as f64 / sf);
    let cy = (oy + finite(wave.cy)).clamp(0.0, size.height as f64 / sf);
    let duration = wave.duration.clamp(300, 6000);
    let bulge = if wave.bulge.is_finite() { wave.bulge.clamp(0.0, 40.0) } else { 0.0 };

    // a captura vem antes de mostrar a janela da onda, senão ela sairia na foto
    let mut capture = false;
    if bulge > 0.0 {
        let (x, y, w, h) = (pos.x, pos.y, size.width as i32, size.height as i32);
        let frame = blocking(move || platform::capture_screen(x, y, w, h)).await?;
        capture = frame.is_some();
        *shared.wave_frame.lock().unwrap() = frame;
    }

    let _ = wv.set_position(pos);
    // 1 px mais baixa que o monitor: uma janela "sempre no topo" que cobre o
    // monitor inteiro o Windows conta como app em tela cheia, e a ilha (e o
    // que mais consulta isso, como as notificações) se esconderia na onda
    let size_wave = PhysicalSize::new(size.width, size.height.saturating_sub(1));
    let _ = wv.set_size(size_wave);
    // mostra sem ativar e logo abaixo da ilha, que continua por cima da onda
    if let (Ok(a), Ok(b)) = (wv.hwnd(), main.hwnd()) {
        platform::show_below(a.0 as isize, b.0 as isize);
    }
    let _ = app.emit_to(
        "wave",
        "wave:play",
        WavePlay {
            color: wave.color,
            intensity: if wave.intensity == "medium" { "medium".into() } else { "subtle".into() },
            duration,
            bulge: if capture { bulge } else { 0.0 },
            cx,
            cy,
            width: size_wave.width,
            height: size_wave.height,
            scale: sf,
            capture,
            at,
        },
    );

    // prazo de segurança: se a página da onda não avisar o fim (recarregou,
    // travou), a janela some do mesmo jeito e não bloqueia as próximas
    let gen = shared.wave_gen.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(duration as u64 + 4000));
        if shared.wave_gen.load(Ordering::SeqCst) == gen && shared.wave_on.load(Ordering::SeqCst) {
            finish_wave(&app, &shared);
        }
    });
    Ok(())
}

/// Captura da onda atual em bytes crus (ver `platform::capture_screen`); vazia
/// se não houver. Sai do `Shared` ao entregar: o buffer é liberado na hora.
#[tauri::command]
fn wave_frame(shared: tauri::State<'_, Arc<Shared>>) -> tauri::ipc::Response {
    tauri::ipc::Response::new(shared.wave_frame.lock().unwrap().take().unwrap_or_default())
}

/// A página da onda terminou de animar (e já limpou o quadro).
#[tauri::command]
fn wave_done(app: AppHandle, shared: tauri::State<'_, Arc<Shared>>) {
    finish_wave(&app, &shared);
}

/// Janela da onda: cobre o monitor da ilha, transparente, sem borda nem foco,
/// fora da barra de tarefas e com o clique atravessando. Criada escondida no
/// início, para a onda não esperar o WebView carregar. Daí em diante quem a
/// mostra e esconde é o Win32 (`platform::show_below`/`hide_window`), nunca o
/// `show()`/`hide()` do Tauri, que a ativaria.
fn build_wave_window(app: &tauri::App) -> tauri::Result<()> {
    let w = tauri::WebviewWindowBuilder::new(app, "wave", tauri::WebviewUrl::App("wave.html".into()))
        .title("Island")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
    let _ = w.set_ignore_cursor_events(true);
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, SettingsState>) -> settings::Settings {
    state.cur.lock().unwrap().clone()
}

/// Grava as configurações do painel e aplica o que é do lado do Rust.
/// Devolve a versão corrigida (`sanitize`) para o frontend usar.
#[tauri::command]
fn set_settings(
    app: AppHandle,
    state: tauri::State<'_, SettingsState>,
    shared: tauri::State<'_, Arc<Shared>>,
    settings: settings::Settings,
) -> Result<settings::Settings, String> {
    let mut s = settings;
    s.sanitize();
    let old = state.cur.lock().unwrap().clone();
    settings::save(&state.file, &s).map_err(|e| e.to_string())?;
    *state.cur.lock().unwrap() = s.clone();
    shared.hide_osd.store(s.hide_volume_osd && s.modules.volume, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window("main") {
        if old.monitor != s.monitor {
            *shared.monitor.lock().unwrap() = s.monitor.clone();
            place_island(&w, &shared);
        }
        // a página já redesenhou a pílula na posição nova; aqui só a janela muda
        if old.position != s.position {
            *shared.anchor.lock().unwrap() = settings::anchor(&s.position);
            place_island(&w, &shared);
        }
        if (old.scale - s.scale).abs() > f64::EPSILON {
            apply_scale(&w, &shared, s.scale);
        }
    }
    // sempre: também retoma os atalhos depois de uma gravação no painel
    shortcuts::apply(&app, &s.shortcuts);
    dock::apply(&app, &s.taskbar);
    Ok(s)
}

/// Escala da ilha: a janela cresce e o WebView ganha o mesmo zoom, então a
/// página continua vendo um palco de 760×500 px CSS.
fn apply_scale(win: &WebviewWindow, shared: &Shared, scale: f64) {
    *shared.zoom.lock().unwrap() = scale;
    let _ = win.set_size(LogicalSize::new(BASE_W * scale, BASE_H * scale));
    let _ = win.set_zoom(scale);
    place_island(win, shared);
}

#[derive(Serialize)]
struct MonitorInfo {
    id: String,
    width: u32,
    height: u32,
    primary: bool,
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let Some(w) = app.get_webview_window("main") else { return vec![] };
    let primary = w.primary_monitor().ok().flatten().and_then(|m| m.name().cloned());
    w.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = m.name()?.clone();
            Some(MonitorInfo { primary: primary.as_ref() == Some(&id), width: m.size().width, height: m.size().height, id })
        })
        .collect()
}

/// Painel gravando uma combinação: solta os atalhos para a tecla chegar à
/// página; ao cancelar, registra de novo os que estão salvos.
#[tauri::command]
fn pause_shortcuts(app: AppHandle, state: tauri::State<'_, SettingsState>, paused: bool) {
    if paused {
        shortcuts::pause(&app);
    } else {
        let s = state.cur.lock().unwrap().shortcuts.clone();
        shortcuts::apply(&app, &s);
    }
}

#[tauri::command]
fn check_update(app: AppHandle) {
    update::tray_clicked(&app);
}

#[tauri::command]
fn open_path(path: String) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&path).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
}

// ------------------------------------------------------------- threads

/// Monitor escolhido; se ele não estiver conectado (ou for "automático"), o principal.
fn target_monitor(win: &WebviewWindow, shared: &Shared) -> Option<tauri::Monitor> {
    let want = shared.monitor.lock().unwrap().clone();
    want.and_then(|name| {
        win.available_monitors()
            .ok()?
            .into_iter()
            .find(|m| m.name().is_some_and(|n| *n == name))
    })
    .or_else(|| win.primary_monitor().ok().flatten())
    .or_else(|| win.current_monitor().ok().flatten())
}

/// Posição (física) que a janela deve ocupar no monitor, conforme a posição
/// escolhida: encostada no topo ou na base e centralizada ou encostada num lado.
/// Usa a área de trabalho (sem a barra de tarefas): na base a ilha fica logo
/// acima dela. A pílula fica a 6 px da borda de cima/baixo e, nos cantos, a
/// `SIDE_GAP` da lateral; isso o CSS resolve dentro da janela.
/// O tamanho sai da escala do monitor de destino, não do tamanho atual: ao
/// trocar para um monitor com outro DPI o Windows redimensiona a janela.
fn target_position(win: &WebviewWindow, shared: &Shared) -> Option<PhysicalPosition<i32>> {
    let m = target_monitor(win, shared)?;
    let k = shared.zoom() * m.scale_factor();
    let (w, h) = ((BASE_W * k).round() as i32, (BASE_H * k).round() as i32);
    let wa = m.work_area();
    let (ax, ay) = (wa.position.x, wa.position.y);
    let (aw, ah) = (wa.size.width as i32, wa.size.height as i32);
    let (bottom, side) = *shared.anchor.lock().unwrap();
    let x = match side {
        -1 => ax,
        1 => ax + aw - w,
        _ => ax + (aw - w) / 2,
    };
    let y = if bottom { ay + ah - h } else { ay };
    Some(PhysicalPosition::new(x, y))
}

fn place_island(win: &WebviewWindow, shared: &Shared) {
    if let Some(p) = target_position(win, shared) {
        let _ = win.set_position(p);
    }
}

/// Área da pílula compacta com folga (a largura do maior aviso, 340 px CSS, e
/// 44 px de altura), em px físicos da tela, para saber se um app está por baixo
/// da ilha. Segue a posição: faixa no topo ou na base da janela; centralizada ou,
/// nos cantos, saindo da lateral da janela (a pílula fica a `SIDE_GAP` dela).
/// `k` = px físicos por px CSS (escala do monitor × zoom da ilha).
fn pill_zone(p: PhysicalPosition<i32>, sz: PhysicalSize<u32>, k: f64, (bottom, side): (bool, i32)) -> (i32, i32, i32, i32) {
    let (w, h) = (sz.width as i32, sz.height as i32);
    let reach = ((SIDE_GAP + 340.0) * k) as i32;
    let (l, r) = match side {
        -1 => (p.x, p.x + reach),
        1 => (p.x + w - reach, p.x + w),
        _ => {
            let (cx, half) = (p.x + w / 2, (170.0 * k) as i32);
            (cx - half, cx + half)
        }
    };
    let band = (44.0 * k) as i32;
    let (t, b) = if bottom { (p.y + h - band, p.y + h) } else { (p.y, p.y + band) };
    (l, t, r, b)
}

/// ~30 Hz: hover/click-through, volume (vindo do callback), Caps Lock,
/// clipboard, tela cheia.
fn spawn_fast(app: AppHandle, win: WebviewWindow, shared: Arc<Shared>, vol: platform::Volume) {
    thread::spawn(move || {
        let fast = platform::Fast::new();
        let mut inside_last = false;
        let mut last_fast: Option<(i32, bool, bool)> = None;
        let mut clip_seq = fast.clipboard_seq();
        let mut hidden = false;
        let mut covered_last = false;
        let own_hwnd = win.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        let mut tick: u64 = 0;
        let _ = win.set_ignore_cursor_events(true);

        loop {
            tick += 1;

            // hover: cursor dentro da pílula?
            let inside = match (win.cursor_position(), win.outer_position(), win.scale_factor()) {
                (Ok(c), Ok(p), Ok(s)) => {
                    let r = *shared.rect.lock().unwrap();
                    // o rect vem em px CSS; com zoom do WebView (escala da ilha)
                    // 1 px CSS = zoom px lógicos
                    let s = s * shared.zoom();
                    let lx = (c.x - p.x as f64) / s;
                    let ly = (c.y - p.y as f64) / s;
                    let pad = 4.0;
                    lx >= r.x - pad && lx <= r.x + r.w + pad && ly >= r.y - pad && ly <= r.y + r.h + pad
                }
                _ => false,
            };
            if inside != inside_last {
                inside_last = inside;
                shared.hover.store(inside, Ordering::Relaxed);
                let _ = win.set_ignore_cursor_events(!inside);
                let _ = app.emit("island:hover", inside);
            }

            // volume na volta seguinte ao callback do Core Audio; Caps Lock
            // a cada ~100 ms
            if shared.vol_dirty.swap(false, Ordering::Relaxed) || tick % 3 == 0 {
                let (volume, muted) = match vol.get() {
                    Some((v, m)) => (Some(v), m),
                    None => (None, false),
                };
                let caps = fast.caps_lock();
                let key = ((volume.unwrap_or(-1.0) * 100.0).round() as i32, muted, caps);
                if last_fast != Some(key) {
                    last_fast = Some(key);
                    let _ = app.emit("island:fast", Fast { volume, muted, caps });
                }
            }

            // clipboard a cada ~330 ms
            if tick % 10 == 0 {
                let seq = fast.clipboard_seq();
                if seq != clip_seq {
                    clip_seq = seq;
                    if seq != shared.own_clip_seq.load(Ordering::SeqCst) {
                        if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                            if !text.trim().is_empty() {
                                let t: String = text.chars().take(2000).collect();
                                let _ = app.emit("island:clip", t);
                            }
                        }
                    }
                }
            }

            // app cobrindo a ilha (tela cheia, ou janela maximizada/grande por
            // baixo dela, ex.: abas do navegador): o frontend esconde a pílula
            // (fica uma faixa invisível na borda da tela, topo ou base, para
            // puxá-la) ou a reduz a um traço. Só tela cheia exclusiva (jogo D3D, apresentação) some de
            // verdade. ~330 ms para reagir rápido a maximizar/Alt+Tab.
            if tick % 10 == 0 {
                let (fs, exclusive) = fast.fullscreen();
                if shared.fullscreen.swap(fs, Ordering::Relaxed) != fs {
                    dock::fullscreen(&app, fs);
                }
                let cov = match (win.outer_position(), win.outer_size(), win.scale_factor()) {
                    (Ok(p), Ok(sz), Ok(sf)) => {
                        let zone = pill_zone(p, sz, sf * shared.zoom(), *shared.anchor.lock().unwrap());
                        fast.covering(own_hwnd, zone)
                    }
                    _ => Some(false),
                };
                // None = a própria ilha em primeiro plano (clicaram nela): mantém
                let covered = fs || cov.unwrap_or(covered_last);
                if exclusive != hidden {
                    hidden = exclusive;
                    let _ = if exclusive { win.hide() } else { win.show() };
                }
                if covered != covered_last {
                    covered_last = covered;
                    shared.covered.store(covered, Ordering::Relaxed);
                    let _ = app.emit("island:covered", covered);
                }
            }

            // monitor conectado/desconectado (ou resolução mudou): ~a cada 2 s
            // confere onde a ilha deveria estar e só move se mudou de verdade
            if tick % 60 == 0 {
                if let (Some(want), Ok(now)) = (target_position(&win, &shared), win.outer_position()) {
                    if (want.x - now.x).abs() > 2 || want.y != now.y {
                        let _ = win.set_position(want);
                    }
                }
            }

            thread::sleep(Duration::from_millis(33));
        }
    });
}

/// Volume pelo callback do Core Audio: marca para o `spawn_fast` emitir e, se
/// alguém mexeu no volume com o HUD da ilha na tela, acorda na hora a thread
/// que esconde o aviso do Windows (sem HUD, o do Windows fica).
fn start_volume(shared: Arc<Shared>) -> platform::Volume {
    platform::Volume::start(move |key| {
        if key
            && shared.hide_osd.load(Ordering::Relaxed)
            && !shared.covered.load(Ordering::Relaxed)
            && !shared.hover.load(Ordering::Relaxed)
            && !shared.hud_blocked.load(Ordering::Relaxed)
        {
            shared.wake_osd();
        }
        shared.vol_dirty.store(true, Ordering::Relaxed);
    })
}

/// Esconde o aviso de volume do próprio Windows. Dorme até o callback de
/// volume avisar (`Shared::wake_osd`) que o volume mudou com o HUD da ilha na tela;
/// aí, por ~400 ms, esconde a janela do aviso assim que o Explorer a mostra
/// (ele a mostra de novo a cada tecla). Ociosa não acorda nunca. Com a opção
/// desligada (ou ao sair) ninguém a acorda e o Windows volta a mostrar o aviso
/// na próxima tecla.
fn spawn_volume_osd(shared: Arc<Shared>) {
    thread::spawn(move || {
        let _ = shared.osd_thread.set(thread::current());
        let mut osd = platform::VolumeOsd::new();
        loop {
            thread::park();
            while shared.hide_osd.load(Ordering::Relaxed)
                && shared.osd_at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400))
            {
                if osd.visible() {
                    osd.hide();
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    });
}

/// Mídia: consulta a cada 700 ms, emite só quando muda.
fn spawn_media(app: AppHandle) {
    thread::spawn(move || {
        let mut media = platform::Media::new();
        let mut last: Option<platform::MediaInfo> = None;
        let mut last_track = String::new();
        let mut first = true;
        // navegadores entregam a capa com atraso: tenta de novo por ~15 s
        let mut art_tries: u32 = 0;
        loop {
            let snap = media.snapshot();
            let track = snap.as_ref().map(|m| format!("{}\u{1}{}", m.title, m.artist)).unwrap_or_default();

            let changed = match (&snap, &last) {
                (Some(a), Some(b)) => {
                    a.title != b.title
                        || a.artist != b.artist
                        || a.playing != b.playing
                        || (a.duration - b.duration).abs() > 0.5
                        // salto de posição (seek) maior que o esperado
                        || (a.position - b.position).abs() > 2.5 && a.updated_ms != b.updated_ms
                }
                (None, None) => false,
                _ => true,
            };
            if changed || first {
                let _ = app.emit("island:media", &snap);
            }
            if track != last_track {
                last_track = track;
                art_tries = 0;
                if snap.is_some() {
                    thread::sleep(Duration::from_millis(250));
                    let _ = media.snapshot();
                    let art = media.artwork();
                    if art.is_none() {
                        art_tries = 1;
                    }
                    let _ = app.emit("island:art", art);
                }
            } else if art_tries > 0 && art_tries <= 20 && snap.is_some() {
                art_tries += 1;
                if let Some(art) = media.artwork() {
                    art_tries = 0;
                    let _ = app.emit("island:art", Some(art));
                }
            }
            first = false;
            last = snap;
            thread::sleep(Duration::from_millis(700));
        }
    });
}

/// Sistema, bateria e rede a cada 1 s.
fn spawn_slow(app: AppHandle, shared: Arc<Shared>) {
    thread::spawn(move || {
        platform::init_thread();
        let mut mon = sys::Monitor::new();
        let mut net = platform::network();
        let mut t = Instant::now();
        let mut tick: u64 = 0;
        loop {
            thread::sleep(Duration::from_millis(1000));
            let dt = t.elapsed().as_secs_f64();
            t = Instant::now();
            tick += 1;
            if tick % 3 == 0 {
                net = platform::network();
            }
            let stats = mon.sample(dt);
            let kind = shared.detail.lock().unwrap().clone();
            let detail = kind.map(|k| mon.detail(&k));
            let _ = app.emit(
                "island:slow",
                Slow { stats, battery: platform::battery(), network: net.clone(), detail },
            );
        }
    });
}

/// Brilho da tela interna: assina o aviso de energia e o evento do WMI (zero
/// custo parado, o que chegar primeiro vale) e emite só quando muda, nunca na
/// leitura inicial. Quem liga/desliga o aviso é o frontend (`modules.brightness`).
/// Sem tela interna, ou se a primeira leitura falha (driver ainda não pronto,
/// tampa fechada no login) ou as duas assinaturas quebram, tenta de novo a cada
/// 60 s, sem nunca girar em loop rápido.
fn spawn_brightness(app: AppHandle) {
    thread::spawn(move || {
        platform::init_thread();
        loop {
            // `watch()` só depois de `read()` confirmar a tela interna: no desktop
            // não assina nada nem sobe a thread do WMI a cada nova tentativa
            if let Some((mut last, mut watch)) =
                platform::Brightness::new().and_then(|br| Some((br.read()?, br.watch()?)))
            {
                while let Some(now) = watch.next() {
                    if now != last {
                        last = now;
                        let _ = app.emit("island:brightness", now);
                    }
                }
            }
            thread::sleep(Duration::from_secs(60));
        }
    });
}

/// Cor de destaque do Windows (aura "Sistema"): assina o `ColorValuesChanged`
/// antes da leitura inicial, para não perder uma troca no meio, e depois só
/// acorda quando o Windows avisa. Emite `island:accent` só se a cor mudou de
/// verdade (o aviso também vem por tema claro/escuro). Sem assinatura, fica a
/// leitura inicial, que o comando `accent_color` entrega.
fn spawn_accent(app: AppHandle, shared: Arc<Shared>) {
    thread::spawn(move || {
        platform::init_thread();
        let Some(acc) = platform::Accent::new() else { return };
        let watch = acc.watch();
        *shared.accent.lock().unwrap() = acc.read();
        let Some(watch) = watch else { return };
        while let Some(now) = watch.next() {
            let mut cur = shared.accent.lock().unwrap();
            if cur.as_deref() != Some(now.as_str()) {
                *cur = Some(now.clone());
                drop(cur);
                let _ = app.emit("island:accent", now);
            }
        }
    });
}

/// Dispositivo USB conectado: dorme no aviso do cfgmgr32 (zero custo parado)
/// e manda o nome para a ilha, que decide o aviso e a onda (`modules.usb`,
/// `usb_wave`). Se a assinatura falhar, tenta de novo a cada 60 s.
fn spawn_usb(app: AppHandle) {
    thread::spawn(move || loop {
        if let Some(watch) = platform::UsbWatch::start() {
            while let Some(name) = watch.next() {
                let _ = app.emit_to("main", "island:usb", name);
            }
        }
        thread::sleep(Duration::from_secs(60));
    });
}

/// Notificações de outros apps (precisa de identidade de pacote).
fn spawn_notifs(app: AppHandle) {
    thread::spawn(move || {
        platform::init_thread();
        // dá tempo do frontend carregar antes do primeiro status
        thread::sleep(Duration::from_secs(2));
        let mut status = platform::notif_access(true);
        let _ = app.emit("island:notif-status", &status);
        loop {
            if status != "allowed" {
                thread::sleep(Duration::from_secs(5));
                let st = platform::notif_access(false);
                if st != status {
                    status = st;
                    let _ = app.emit("island:notif-status", &status);
                }
                continue;
            }
            let Some(mut n) = platform::Notifs::new() else {
                status = "error".into();
                continue;
            };
            loop {
                for info in n.poll() {
                    let _ = app.emit("island:notif", info);
                }
                thread::sleep(Duration::from_millis(900));
            }
        }
    });
}

/// Na primeira execução liga "Iniciar com o Windows" (padrão, como o
/// TranslucentTB). O marcador registra que isso já foi feito, para respeitar
/// quem desligar depois pela bandeja. O desinstalador apaga os dois.
/// Em debug não mexe: o Run apontaria para o exe de target\debug.
fn autostart_first_run(app: &tauri::App) {
    if cfg!(debug_assertions) {
        return;
    }
    let Ok(dir) = app.path().app_config_dir() else { return };
    let marker = dir.join("autostart-default");
    if marker.exists() {
        return;
    }
    if app.autolaunch().enable().is_ok() {
        platform::autostart_quote_path();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&marker, b"1");
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let auto = CheckMenuItem::with_id(app, "autostart", "Iniciar com o Windows", true, autostart_on, None::<&str>)?;
    let center = MenuItem::with_id(app, "center", "Reposicionar", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Recarregar visual", true, None::<&str>)?;
    let prefs = MenuItem::with_id(app, "settings", "Configurações", true, None::<&str>)?;
    let upd = MenuItem::with_id(app, "update", "Procurar atualizações", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&prefs, &auto, &center, &reload, &upd, &sep, &quit])?;
    update::set_tray_item(app.handle(), upd);

    let auto_item = auto.clone();
    TrayIconBuilder::with_id("island")
        .tooltip("Island")
        .icon(app.default_window_icon().cloned().expect("ícone"))
        .menu(&menu)
        .on_menu_event(move |app, ev| match ev.id().as_ref() {
            "quit" => app.exit(0),
            "center" => {
                if let Some(w) = app.get_webview_window("main") {
                    place_island(&w, &app.state::<Arc<Shared>>());
                }
            }
            "reload" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.eval("location.reload()");
                }
            }
            "update" => update::tray_clicked(app),
            "settings" => {
                let _ = app.emit("island:open-settings", ());
            }
            "autostart" => {
                let al = app.autolaunch();
                let on = al.is_enabled().unwrap_or(false);
                let _ = if on { al.disable() } else { al.enable() };
                if !on {
                    platform::autostart_quote_path();
                }
                let _ = auto_item.set_checked(al.is_enabled().unwrap_or(false));
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    // `island.exe --taskbar-watchdog`: só o vigia que devolve a barra nativa
    if dock::watchdog_main() {
        return;
    }
    let shared = Arc::new(Shared::default());
    let on_exit = shared.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(shortcuts::plugin())
        .manage(shared.clone())
        .manage(update::Updates::default())
        .manage(shortcuts::Bindings::default())
        .invoke_handler(tauri::generate_handler![
            set_hit_rect, set_hud_blocked, media_cmd, copy_text, open_path, set_detail, wifi_list, wifi_connect,
            power_info, set_power_plan, set_power_mode, open_uri, enable_notifications,
            update::pending_update, update::install_update, check_update, get_settings, set_settings,
            list_monitors, pause_shortcuts, shortcuts::shortcut_errors, user_info, power_action, accent_color,
            play_wave, wave_frame, wave_done,
            dock::dock_init, dock::dock_layout, dock::dock_click, dock::dock_action, dock::dock_icon
        ])
        .setup(move |app| {
            let win = app.get_webview_window("main").expect("janela main");

            let file = settings::path(&app.path().app_config_dir()?);            let prefs = settings::load(&file);
            *shared.monitor.lock().unwrap() = prefs.monitor.clone();
            *shared.anchor.lock().unwrap() = settings::anchor(&prefs.position);
            shared.hide_osd.store(prefs.hide_volume_osd && prefs.modules.volume, Ordering::Relaxed);
            apply_scale(&win, &shared, prefs.scale);
            shortcuts::apply(app.handle(), &prefs.shortcuts);
            dock::setup(app.handle(), &prefs.taskbar);
            app.manage(SettingsState { cur: Mutex::new(prefs), file });

            let _ = win.show();

            // arrastar arquivos para a pílula -> estante
            let handle = app.handle().clone();
            win.on_window_event(move |ev| {
                if let WindowEvent::DragDrop(dd) = ev {
                    let _ = match dd {
                        DragDropEvent::Enter { .. } => handle.emit("island:drag", true),
                        DragDropEvent::Leave => handle.emit("island:drag", false),
                        DragDropEvent::Drop { paths, .. } => {
                            let list: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                            let _ = handle.emit("island:drag", false);
                            handle.emit("island:drop", list)
                        }
                        _ => Ok(()),
                    };
                }
            });

            autostart_first_run(app); // antes da bandeja, para o check já vir marcado
            // 0.4.0 gravou o Run sem aspas; só reescreve se já existir, então autostart desligado continua desligado
            #[cfg(not(debug_assertions))]
            platform::autostart_quote_path();
            build_tray(app)?;
            build_wave_window(app)?;
            spawn_volume_osd(shared.clone());
            let vol = start_volume(shared.clone());
            spawn_fast(app.handle().clone(), win, shared.clone(), vol);
            spawn_media(app.handle().clone());
            spawn_slow(app.handle().clone(), shared.clone());
            spawn_notifs(app.handle().clone());
            spawn_brightness(app.handle().clone());
            spawn_accent(app.handle().clone(), shared.clone());
            spawn_usb(app.handle().clone());
            update::spawn(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Island")
        .run(move |_, ev| {
            // ao sair: para de esconder o aviso de volume do Windows
            if let tauri::RunEvent::Exit = ev {
                on_exit.hide_osd.store(false, Ordering::Relaxed);
                // a barra nativa volta antes de o processo terminar
                dock::shutdown();
            }
        });
}
