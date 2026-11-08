#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod sys;

use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};
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
}

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

#[tauri::command]
fn open_path(path: String) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&path).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
}

// ------------------------------------------------------------- threads

fn place_top_center(win: &WebviewWindow) {
    let mon = win.current_monitor().ok().flatten().or_else(|| win.primary_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (mon, win.outer_size()) {
        let x = m.position().x + (m.size().width as i32 - size.width as i32) / 2;
        let _ = win.set_position(PhysicalPosition::new(x, m.position().y));
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
        let mut tick: u64 = 0;
        let _ = win.set_ignore_cursor_events(true);

        loop {
            tick += 1;

            // hover: cursor dentro da pílula?
            let inside = match (win.cursor_position(), win.outer_position(), win.scale_factor()) {
                (Ok(c), Ok(p), Ok(s)) => {
                    let r = *shared.rect.lock().unwrap();
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

            // some em jogos/vídeos em tela cheia
            if tick % 30 == 0 {
                let fs = fast.fullscreen();
                if fs != hidden {
                    hidden = fs;
                    let _ = if fs { win.hide() } else { win.show() };
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

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let auto = CheckMenuItem::with_id(app, "autostart", "Iniciar com o Windows", true, autostart_on, None::<&str>)?;
    let center = MenuItem::with_id(app, "center", "Recentralizar", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Recarregar visual", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&auto, &center, &reload, &sep, &quit])?;

    let auto_item = auto.clone();
    TrayIconBuilder::with_id("island")
        .tooltip("Island")
        .icon(app.default_window_icon().cloned().expect("ícone"))
        .menu(&menu)
        .on_menu_event(move |app, ev| match ev.id().as_ref() {
            "quit" => app.exit(0),
            "center" => {
                if let Some(w) = app.get_webview_window("main") {
                    place_top_center(&w);
                }
            }
            "reload" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.eval("location.reload()");
                }
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
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![
            set_hit_rect, media_cmd, copy_text, open_path, set_detail, wifi_list, wifi_connect,
            power_info, set_power_plan, set_power_mode, open_uri
        ])
        .setup(move |app| {
            let win = app.get_webview_window("main").expect("janela main");
            place_top_center(&win);
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

            build_tray(app)?;
            spawn_fast(app.handle().clone(), win, shared.clone());
            spawn_media(app.handle().clone());
            spawn_slow(app.handle().clone(), shared.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Island");
}
