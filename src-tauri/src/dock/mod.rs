//! Módulo "Barra de tarefas": o Island desenha a própria barra, uma dock
//! flutuante de vidro, e esconde a barra nativa do Windows enquanto o módulo
//! está ligado (`settings.taskbar.enabled`). Sem relógio, Wi-Fi e bateria: a
//! ilha já mostra isso.
//!
//! Peças (só existem com o módulo ligado):
//! - janela `dock` (WebView, `ui/dock.html`): transparente, sem borda, sempre
//!   no topo, fora da barra, sem foco; ocupa a largura do monitor principal e
//!   a altura da dock com folga para a ampliação e as dicas. Fora da dock os
//!   cliques atravessam (`set_ignore_cursor_events`, como a ilha);
//! - thread `dock-hover`: hit-test do cursor contra o retângulo que a página
//!   informa (`dock_layout`); acorda a cada 150 ms longe da dock e a cada
//!   20 ms perto dela;
//! - thread `dock-shell` (`win.rs`): esconde a barra nativa, reserva a faixa
//!   como AppBar, lista as janelas por evento, vidro desfocado por baixo da dock;
//! - thread `dock-icons`: extrai os ícones reais (sob demanda, com cache);
//! - processo vigia (`taskbar.rs`): devolve a barra nativa se o Island morrer.
//!
//! Ligar/desligar roda numa thread à parte (`reconcile`): criar janela num
//! comando síncrono trava o Tauri no Windows, e duas trocas seguidas no
//! painel se resolvem pela última configuração.

use crate::settings::Taskbar;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

#[cfg(windows)]
mod taskbar;
#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as sys;
#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as sys;

/// Altura da janela da dock em px CSS: dock (até 48 + 14) + margem de 10 +
/// ampliação forte (+60%) + dica, com sobra para a sombra (o pior caso, Grande
/// com Forte, chega a uns 155).
const DOCK_H: f64 = 190.0;
/// Margem entre a dock e o pé da tela, e folga acima dela na faixa reservada
/// (px CSS; os mesmos de `ui/dock.css`).
const BOTTOM_GAP: f64 = 10.0;
const TOP_GAP: f64 = 4.0;
/// Espaço interno da dock acima e abaixo do ícone (px CSS)
const DOCK_PAD: f64 = 7.0;

/// Botão da dock como a página o vê.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct ItemOut {
    pub key: String,
    pub name: String,
    pub pinned: bool,
    /// janelas abertas
    pub open: usize,
    /// uma delas está em primeiro plano
    pub focus: bool,
}

/// Retângulo em px CSS relativo à janela da dock.
#[derive(Deserialize, Clone, Copy, Default, Debug)]
pub struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Monitor principal em px físicos.
#[derive(Clone, Copy, Debug)]
struct Geo {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scale: f64,
}

impl Geo {
    fn dock_h(&self) -> i32 {
        (DOCK_H * self.scale).round() as i32
    }
    /// Canto superior esquerdo da janela da dock (colada no pé do monitor).
    fn origin(&self) -> (i32, i32) {
        (self.x, self.y + self.h - self.dock_h())
    }
    /// Faixa reservada como AppBar: a dock e as margens dela.
    fn appbar(&self, cfg: &Taskbar) -> (i32, i32, i32, i32) {
        let css = BOTTOM_GAP + DOCK_PAD * 2.0 + cfg.icon_px() + TOP_GAP;
        let px = (css * self.scale).round() as i32;
        (self.x, self.y + self.h - px, self.x + self.w, self.y + self.h)
    }
}

/// Hit-test do cursor (thread `dock-hover`).
struct Hover {
    stop: AtomicBool,
    /// área clicável em px CSS (a dock e os ícones ampliados)
    hit: Mutex<Option<Rect>>,
    visible: AtomicBool,
    geo: Mutex<Geo>,
    thread: OnceLock<std::thread::Thread>,
}

struct Run {
    win: WebviewWindow,
    engine: sys::Engine,
    geo: Geo,
    hover: Arc<Hover>,
    cfg: Taskbar,
}

struct State {
    app: Option<AppHandle>,
    cfg: Taskbar,
    marker: Option<PathBuf>,
    run: Option<Run>,
}

static STATE: LazyLock<Mutex<State>> =
    LazyLock::new(|| Mutex::new(State { app: None, cfg: Taskbar::default(), marker: None, run: None }));
/// Serializa ligar/desligar/reposicionar (nunca segurado por comandos).
static TRANSITION: Mutex<()> = Mutex::new(());
static FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// O exe foi chamado como processo vigia da barra nativa? Aí faz só isso e
/// sai (antes do Tauri subir qualquer coisa).
pub fn watchdog_main() -> bool {
    #[cfg(windows)]
    return taskbar::watchdog_main();
    #[cfg(not(windows))]
    false
}

/// Início do app: devolve a barra nativa se uma execução anterior morreu com
/// ela escondida e liga a dock se estiver configurada.
pub fn setup(app: &AppHandle, cfg: &Taskbar) {
    let marker = app.path().app_config_dir().ok().map(|d| d.join("taskbar-restore.json"));
    #[cfg(windows)]
    if let Some(m) = &marker {
        taskbar::restore_from(m);
    }
    {
        let mut s = STATE.lock().unwrap();
        s.app = Some(app.clone());
        s.cfg = cfg.clone();
        s.marker = marker;
    }
    if cfg.enabled {
        spawn_reconcile(false);
    }
}

/// Configurações gravadas no painel: liga, desliga ou atualiza a dock na hora.
pub fn apply(_app: &AppHandle, cfg: &Taskbar) {
    let changed = {
        let mut s = STATE.lock().unwrap();
        let changed = s.cfg != *cfg;
        s.cfg = cfg.clone();
        changed
    };
    if changed {
        spawn_reconcile(false);
    }
}

/// App em tela cheia (o `spawn_fast` avisa quando muda): a dock some.
pub fn fullscreen(_app: &AppHandle, on: bool) {
    FULLSCREEN.store(on, Ordering::SeqCst);
    // copia e solta a trava antes de falar com a janela (o Tauri espera a
    // thread principal, que pode estar esperando esta trava ao sair)
    let parts = STATE.lock().unwrap().run.as_ref().map(Run::parts);
    if let Some(p) = parts {
        set_shown(&p, !on);
    }
}

/// Saída do Island: desfaz tudo na hora (a barra nativa volta) sem esperar
/// a thread de transição nem mexer na janela.
pub fn shutdown() {
    let run = STATE.lock().unwrap().run.take();
    if let Some(r) = run {
        r.hover.stop.store(true, Ordering::SeqCst);
        r.engine.stop();
    }
}

/// O que dá para usar da dock ligada sem segurar a trava do estado.
#[derive(Clone)]
struct Parts {
    win: WebviewWindow,
    engine: sys::Handle,
    geo: Geo,
    hover: Arc<Hover>,
}

impl Run {
    fn parts(&self) -> Parts {
        Parts { win: self.win.clone(), engine: self.engine.h.clone(), geo: self.geo, hover: self.hover.clone() }
    }
}

/// Esconde/mostra a dock sem `hide()`/`show()` do Tauri: o `show()` ativa a
/// janela (roubaria o foco do app em tela cheia que acabou de fechar). Fora
/// de vista ela vai para longe da área de trabalho e o vidro some.
fn set_shown(p: &Parts, shown: bool) {
    let (x, y) = if shown { p.geo.origin() } else { (-32000, -32000) };
    let _ = p.win.set_position(PhysicalPosition::new(x, y));
    p.engine.set_visible(shown);
    p.hover.visible.store(shown, Ordering::SeqCst);
    if let Some(t) = p.hover.thread.get() {
        t.unpark();
    }
}

fn spawn_reconcile(relayout: bool) {
    let _ = std::thread::Builder::new().name("dock-reconcile".into()).spawn(move || reconcile(relayout));
}

fn primary_geo(app: &AppHandle) -> Option<Geo> {
    let m = app.primary_monitor().ok().flatten()?;
    Some(Geo { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32, scale: m.scale_factor() })
}

/// Leva a dock ao estado da configuração atual.
fn reconcile(relayout: bool) {
    let _t = TRANSITION.lock().unwrap();
    let (app, cfg, marker, running) = {
        let s = STATE.lock().unwrap();
        (s.app.clone(), s.cfg.clone(), s.marker.clone(), s.run.is_some())
    };
    let Some(app) = app else { return };
    if cfg.enabled && !running {
        if let Some(marker) = marker {
            start(&app, &cfg, marker);
        }
    } else if !cfg.enabled && running {
        stop();
    } else if running {
        update(&app, &cfg, relayout);
    }
}

fn start(app: &AppHandle, cfg: &Taskbar, marker: PathBuf) {
    let Some(geo) = primary_geo(app) else { return };
    let on_display: Box<dyn Fn() + Send> = Box::new(|| spawn_reconcile(true));
    let Some(engine) = sys::Engine::start(app.clone(), marker, geo.appbar(cfg), on_display) else { return };

    let (ox, oy) = geo.origin();
    let built = WebviewWindowBuilder::new(app, "dock", WebviewUrl::App("dock.html".into()))
        .title("RAD Dock")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        // sem foco: o clique não tira o primeiro plano do app (focar/minimizar depende dele)
        .focused(false)
        .focusable(false)
        .inner_size(geo.w as f64 / geo.scale, DOCK_H)
        .position(ox as f64 / geo.scale, oy as f64 / geo.scale)
        .build();
    let win = match built {
        Ok(w) => w,
        Err(_) => {
            engine.stop();
            return;
        }
    };
    // px exatos (a posição lógica do builder arredonda em telas com escala)
    let _ = win.set_size(PhysicalSize::new(geo.w as u32, geo.dock_h() as u32));
    let _ = win.set_position(PhysicalPosition::new(ox, oy));
    let _ = win.set_ignore_cursor_events(true);
    #[cfg(windows)]
    if let Ok(h) = win.hwnd() {
        engine.h.set_dock(h.0 as isize);
    }
    let hover = Arc::new(Hover {
        stop: AtomicBool::new(false),
        hit: Mutex::new(None),
        visible: AtomicBool::new(true),
        geo: Mutex::new(geo),
        thread: OnceLock::new(),
    });
    let (h, w) = (hover.clone(), win.clone());
    win.on_window_event(move |ev| match ev {
        WindowEvent::ScaleFactorChanged { .. } => spawn_reconcile(true),
        // quando a faixa reservada cresce, o Windows empurra para cima as
        // janelas que passam do pé da área de trabalho, e a dock é uma delas:
        // volta para o lugar (só se saiu dele, então não entra em laço)
        WindowEvent::Moved(pos) => {
            let want = if h.visible.load(Ordering::SeqCst) { h.geo.lock().unwrap().origin() } else { (-32000, -32000) };
            if (pos.x, pos.y) != want {
                let _ = w.set_position(PhysicalPosition::new(want.0, want.1));
            }
        }
        _ => {}
    });
    spawn_hover(app.clone(), win.clone(), hover.clone());
    let run = Run { win, engine, geo, hover, cfg: cfg.clone() };
    if FULLSCREEN.load(Ordering::SeqCst) {
        set_shown(&run.parts(), false);
    }
    STATE.lock().unwrap().run = Some(run);
}

fn stop() {
    let run = STATE.lock().unwrap().run.take();
    if let Some(r) = run {
        r.hover.stop.store(true, Ordering::SeqCst);
        if let Some(t) = r.hover.thread.get() {
            t.unpark();
        }
        // primeiro a barra nativa volta, depois a dock some
        r.engine.stop();
        let _ = r.win.destroy();
    }
}

/// Módulo já ligado: manda as opções novas à página e, se o tamanho ou o
/// monitor mudou, reposiciona a janela e a faixa reservada.
fn update(app: &AppHandle, cfg: &Taskbar, relayout: bool) {
    let geo = if relayout { primary_geo(app) } else { None };
    let (p, opts, resized) = {
        let mut s = STATE.lock().unwrap();
        let Some(r) = s.run.as_mut() else { return };
        let opts = r.cfg != *cfg;
        let resized = r.cfg.size != cfg.size;
        r.cfg = cfg.clone();
        if let Some(g) = geo {
            r.geo = g;
        }
        (r.parts(), opts, resized)
    };
    if opts {
        let _ = app.emit_to("dock", "dock:cfg", cfg);
    }
    // outro tamanho de ícone só muda a faixa reservada: a janela já tem a
    // largura do monitor e a altura do maior tamanho
    if resized || geo.is_some() {
        p.engine.set_appbar(p.geo.appbar(cfg));
    }
    if geo.is_some() {
        *p.hover.geo.lock().unwrap() = p.geo;
        let _ = p.win.set_size(PhysicalSize::new(p.geo.w as u32, p.geo.dock_h() as u32));
        set_shown(&p, !FULLSCREEN.load(Ordering::SeqCst));
    }
}

/// Click-through fora da dock: a janela só aceita o mouse com o cursor sobre
/// a área que a página informou. Parada (dock escondida) dorme até ser acordada.
fn spawn_hover(app: AppHandle, win: WebviewWindow, h: Arc<Hover>) {
    let _ = std::thread::Builder::new().name("dock-hover".into()).spawn(move || {
        let _ = h.thread.set(std::thread::current());
        let mut inside_last = false;
        while !h.stop.load(Ordering::SeqCst) {
            if !h.visible.load(Ordering::SeqCst) {
                if inside_last {
                    inside_last = false;
                    let _ = win.set_ignore_cursor_events(true);
                    let _ = app.emit_to("dock", "dock:hover", false);
                }
                std::thread::park_timeout(Duration::from_secs(5));
                continue;
            }
            let geo = *h.geo.lock().unwrap();
            let (ox, oy) = geo.origin();
            let (inside, near) = match sys::cursor() {
                Some((cx, cy)) => {
                    let inside = h.hit.lock().unwrap().is_some_and(|r| {
                        let pad = 2.0;
                        let lx = (cx - ox) as f64 / geo.scale;
                        let ly = (cy - oy) as f64 / geo.scale;
                        lx >= r.x - pad && lx <= r.x + r.w + pad && ly >= r.y - pad && ly <= r.y + r.h + pad
                    });
                    (inside, cy >= oy - 40 && cx >= geo.x && cx < geo.x + geo.w)
                }
                None => (false, false),
            };
            if inside != inside_last {
                inside_last = inside;
                let _ = win.set_ignore_cursor_events(!inside);
                let _ = app.emit_to("dock", "dock:hover", inside);
            }
            std::thread::sleep(Duration::from_millis(if inside || near { 20 } else { 150 }));
        }
    });
}

// ------------------------------------------------------------- comandos

#[derive(Serialize)]
pub struct DockInit {
    cfg: Taskbar,
    light: bool,
    items: Vec<ItemOut>,
}

/// Estado inicial da página da dock (depois chegam `dock:items`, `dock:cfg`,
/// `dock:theme` e `dock:hover`).
#[tauri::command]
pub async fn dock_init() -> DockInit {
    let s = STATE.lock().unwrap();
    match &s.run {
        Some(r) => DockInit { cfg: r.cfg.clone(), light: r.engine.h.light(), items: r.engine.h.items() },
        None => DockInit { cfg: s.cfg.clone(), light: false, items: Vec::new() },
    }
}

/// A página mediu a dock: `bar` = o vidro, `hit` = a área clicável (a dock e
/// os ícones ampliados), em px CSS da janela.
#[tauri::command]
pub async fn dock_layout(bar: Rect, hit: Rect) {
    let s = STATE.lock().unwrap();
    let Some(r) = &s.run else { return };
    *r.hover.hit.lock().unwrap() = Some(hit);
    let g = r.geo;
    let (ox, oy) = g.origin();
    let px = |v: f64| (v * g.scale).round() as i32;
    // O desfoque cobre exatamente o vidro, sem recuo. O desfoque do DWM ignora
    // a região da janela (`SetWindowRgn`): os cantos só arredondam pelo canto
    // nativo do Windows 11, de 8 px (que escala com o DPI, como o px CSS). Por
    // isso o vidro da dock tem o mesmo raio de 8 px (`ui/dock.css`): tinta,
    // filete e desfoque coincidem e não aparece um segundo painel por dentro.
    // Arredonda as bordas, não a largura, para não sobrar 1 px de um lado.
    let glass = (bar.w > 0.0 && bar.h > 0.0).then(|| {
        let (l, t, r, b) = (px(bar.x), px(bar.y), px(bar.x + bar.w), px(bar.y + bar.h));
        (ox + l, oy + t, r - l, b - t)
    });
    r.engine.h.set_glass(glass);
}

#[tauri::command]
pub async fn dock_click(key: String) {
    let s = STATE.lock().unwrap();
    if let Some(r) = &s.run {
        r.engine.h.click(&key);
    }
}

/// "start" | "search" | "tray" | "trash"
#[tauri::command]
pub async fn dock_action(kind: String) {
    sys::action(&kind);
}

/// Ícone real do app (data URL PNG) ou da Lixeira (`key` = "trash").
#[tauri::command]
pub async fn dock_icon(key: String) -> Option<String> {
    let srcs = if key == "trash" {
        Some(vec![sys::trash_src()])
    } else {
        STATE.lock().unwrap().run.as_ref().and_then(|r| r.engine.h.icon_srcs(&key))
    }?;
    tauri::async_runtime::spawn_blocking(move || sys::icon(srcs)).await.ok().flatten()
}
