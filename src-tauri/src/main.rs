#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod settings;
mod shortcuts;
mod sys;
mod update;

use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, DragDropEvent, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow, WindowEvent};
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
}

impl Shared {
    fn zoom(&self) -> f64 {
        let z = *self.zoom.lock().unwrap();
        if z > 0.0 { z } else { 1.0 }
    }
}

/// Configurações carregadas + caminho do arquivo.
struct SettingsState {
    cur: Mutex<settings::Settings>,
    file: std::path::PathBuf,
}

const BASE_W: f64 = 760.0;
const BASE_H: f64 = 500.0;

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
    if let Some(w) = app.get_webview_window("main") {
        if old.monitor != s.monitor {
            *shared.monitor.lock().unwrap() = s.monitor.clone();
            place_top_center(&w, &shared);
        }
        if (old.scale - s.scale).abs() > f64::EPSILON {
            apply_scale(&w, &shared, s.scale);
        }
    }
    // sempre: também retoma os atalhos depois de uma gravação no painel
    shortcuts::apply(&app, &s.shortcuts);
    Ok(s)
}

/// Escala da ilha: a janela cresce e o WebView ganha o mesmo zoom, então a
/// página continua vendo um palco de 760×500 px CSS.
fn apply_scale(win: &WebviewWindow, shared: &Shared, scale: f64) {
    *shared.zoom.lock().unwrap() = scale;
    let _ = win.set_size(LogicalSize::new(BASE_W * scale, BASE_H * scale));
    let _ = win.set_zoom(scale);
    place_top_center(win, shared);
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

/// Posição (física) que a janela deve ocupar: topo, centralizada no monitor.
/// A largura sai da escala do monitor de destino, não do tamanho atual: ao
/// trocar para um monitor com outro DPI o Windows redimensiona a janela.
fn target_position(win: &WebviewWindow, shared: &Shared) -> Option<PhysicalPosition<i32>> {
    let m = target_monitor(win, shared)?;
    let w = (BASE_W * shared.zoom() * m.scale_factor()).round() as i32;
    let x = m.position().x + (m.size().width as i32 - w) / 2;
    Some(PhysicalPosition::new(x, m.position().y))
}

fn place_top_center(win: &WebviewWindow, shared: &Shared) {
    if let Some(p) = target_position(win, shared) {
        let _ = win.set_position(p);
    }
}

/// ~30 Hz: hover/click-through, volume, Caps Lock, clipboard, tela cheia.
fn spawn_fast(app: AppHandle, win: WebviewWindow, shared: Arc<Shared>) {
    thread::spawn(move || {
        let mut fast = platform::Fast::new();
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
                let _ = win.set_ignore_cursor_events(!inside);
                let _ = app.emit("island:hover", inside);
            }

            // volume + caps a cada ~100 ms
            if tick % 3 == 0 {
                let (vol, muted) = match fast.volume() {
                    Some((v, m)) => (Some(v), m),
                    None => (None, false),
                };
                let caps = fast.caps_lock();
                let key = ((vol.unwrap_or(-1.0) * 100.0).round() as i32, muted, caps);
                if last_fast != Some(key) {
                    last_fast = Some(key);
                    let _ = app.emit("island:fast", Fast { volume: vol, muted, caps });
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
            // (fica uma faixa invisível no topo para puxá-la) ou a reduz a um
            // traço. Só tela cheia exclusiva (jogo D3D, apresentação) some de
            // verdade. ~330 ms para reagir rápido a maximizar/Alt+Tab.
            if tick % 10 == 0 {
                let (fs, exclusive) = fast.fullscreen();
                let cov = match (win.outer_position(), win.outer_size(), win.scale_factor()) {
                    (Ok(p), Ok(sz), Ok(sf)) => {
                        // área da pílula compacta (com folga), em px físicos
                        let k = sf * shared.zoom();
                        let cx = p.x + sz.width as i32 / 2;
                        let half = (170.0 * k) as i32;
                        fast.covering(own_hwnd, (cx - half, p.y, cx + half, p.y + (44.0 * k) as i32))
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

/// Brilho da tela interna a cada 250 ms (WMI); emite só quando muda, nunca na
/// primeira leitura. Sem tela interna (ou sem WMI) a thread termina em silêncio.
fn spawn_brightness(app: AppHandle) {
    thread::spawn(move || {
        platform::init_thread();
        let Some(br) = platform::Brightness::new() else { return };
        let Some(mut last) = br.read() else { return };
        loop {
            thread::sleep(Duration::from_millis(250));
            if let Some(now) = br.read() {
                if now != last {
                    last = now;
                    let _ = app.emit("island:brightness", now);
                }
            }
        }
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
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&marker, b"1");
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let auto = CheckMenuItem::with_id(app, "autostart", "Iniciar com o Windows", true, autostart_on, None::<&str>)?;
    let center = MenuItem::with_id(app, "center", "Recentralizar", true, None::<&str>)?;
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
                    place_top_center(&w, &app.state::<Arc<Shared>>());
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
                let _ = auto_item.set_checked(al.is_enabled().unwrap_or(false));
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    let shared = Arc::new(Shared::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(shortcuts::plugin())
        .manage(shared.clone())
        .manage(update::Updates::default())
        .manage(shortcuts::Bindings::default())
        .invoke_handler(tauri::generate_handler![
            set_hit_rect, media_cmd, copy_text, open_path, set_detail, wifi_list, wifi_connect,
            power_info, set_power_plan, set_power_mode, open_uri, enable_notifications,
            update::pending_update, update::install_update, check_update, get_settings, set_settings,
            list_monitors, pause_shortcuts, shortcuts::shortcut_errors, user_info, power_action
        ])
        .setup(move |app| {
            let win = app.get_webview_window("main").expect("janela main");

            let file = settings::path(&app.path().app_config_dir()?);
            let prefs = settings::load(&file);
            *shared.monitor.lock().unwrap() = prefs.monitor.clone();
            apply_scale(&win, &shared, prefs.scale);
            shortcuts::apply(app.handle(), &prefs.shortcuts);
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
            build_tray(app)?;
            spawn_fast(app.handle().clone(), win, shared.clone());
            spawn_media(app.handle().clone());
            spawn_slow(app.handle().clone(), shared.clone());
            spawn_notifs(app.handle().clone());
            spawn_brightness(app.handle().clone());
            update::spawn(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Island");
}
