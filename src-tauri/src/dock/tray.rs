//! Bandeja própria do RAD: o botão ⌃ da dock abre os ícones da área de
//! notificação numa janela de vidro (`ui/tray.html`), sem depender da barra
//! nativa escondida.
//!
//! Como os ícones chegam (a técnica dos shells alternativos, RetroBar e
//! ManagedShell): `Shell_NotifyIcon` procura a janela `Shell_TrayWnd` com
//! `FindWindow` e manda `WM_COPYDATA` (dwData 1) para ela. Enquanto a dock está
//! ligada, a thread `dock-tray` mantém uma janela escondida dessa classe no
//! topo da ordem Z, então o `FindWindow` dos apps acha a nossa. Ela anota cada
//! ícone (janela, id, mensagem de retorno, ícone, dica, versão, oculto) e
//! repassa toda mensagem à barra do Explorer, que continua sabendo de tudo:
//! desligar a dock (ou o RAD morrer) não perde ícone nenhum.
//!
//! Ao subir, difunde `TaskbarCreated` (o aviso de "a bandeja renasceu") para os
//! apps registrarem os ícones de novo, agora passando por aqui; ao sair, difunde
//! de novo para eles voltarem a falar direto com o Explorer.
//!
//! Testado no Windows 11 (out/2026): o pacote tem 1484 bytes (cabeçalho de 8,
//! `NOTIFYICONDATA` de 32 bits com 956 e o caminho do exe no fim); o Explorer
//! aceita o repassado, mas responde FALSE a um `NIM_ADD` repetido (o ícone já
//! está lá), e há app que tenta de novo em laço se recebe FALSE. Por isso a
//! resposta ao app é a desta tabela, como uma bandeja recém-criada daria: ADD
//! sempre aceita e MODIFY de ícone desconhecido recusa (a Segurança do Windows
//! só manda MODIFY no `TaskbarCreated` e, recusada, refaz o ADD completo).

use super::taskbar;
use super::TrayIconOut;
use base64::Engine as _;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use windows::core::{w, GUID};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, ChangeWindowMessageFilterEx, CopyIcon, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetIconInfo, GetMessageW, GetWindowRect,
    GetWindowThreadProcessId, InSendMessage, IsWindow, KillTimer, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SendMessageTimeoutW, SetTimer, SetWindowPos, TranslateMessage, HICON, HWND_TOPMOST, ICONINFO,
    MSG, MSGFLT_ALLOW, SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOOWNERZORDER, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU,
    WM_COPYDATA, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_TIMER, WM_USER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

fn hwnd(h: isize) -> HWND {
    HWND(h as *mut _)
}

/// Assinatura do pacote `SHELLTRAYDATA` (dwData 1).
const TRAY_SIG: u32 = 0x3475_3423;
// NIM_*
const NIM_ADD: u32 = 0;
const NIM_MODIFY: u32 = 1;
const NIM_DELETE: u32 = 2;
const NIM_SETFOCUS: u32 = 3;
const NIM_SETVERSION: u32 = 4;
// NIF_*
const NIF_MESSAGE: u32 = 0x01;
const NIF_ICON: u32 = 0x02;
const NIF_TIP: u32 = 0x04;
const NIF_STATE: u32 = 0x08;
const NIF_GUID: u32 = 0x20;
/// NIS_HIDDEN
const NIS_HIDDEN: u32 = 0x01;
/// NIN_SELECT (clique esquerdo na versão 4)
const NIN_SELECT: u32 = WM_USER;

const ID_KEEP: usize = 1;
const ID_NOTIFY: usize = 2;
const ID_REBROADCAST: usize = 3;

/// Um ícone da área de notificação.
struct Icon {
    /// id para a página (não se repete na vida do processo)
    id: u64,
    hwnd: isize,
    uid: u32,
    guid: Option<GUID>,
    /// mensagem que o app quer receber nos cliques (0 = ainda não disse)
    cb: u32,
    /// cópia nossa do ícone (0 = sem ícone)
    hicon: isize,
    tip: String,
    /// `NIM_SETVERSION` (4 = formato novo das mensagens de clique)
    version: u32,
    hidden: bool,
    /// data URL PNG já convertida (`Some(None)` = não deu para converter)
    png: Option<Option<String>>,
}

impl Icon {
    fn free(&mut self) {
        if self.hicon != 0 {
            unsafe {
                let _ = DestroyIcon(HICON(self.hicon as *mut _));
            }
            self.hicon = 0;
        }
    }
}

static TABLE: Mutex<Vec<Icon>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// barra do Explorer, para onde tudo é repassado
static FWD: AtomicIsize = AtomicIsize::new(0);
/// retângulo do botão ⌃ em px físicos (esq, topo, dir, base), para quem
/// pergunta onde fica o ícone (`Shell_NotifyIconGetRect`, dwData 3)
static ANCHOR: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static NOTIFY_ARMED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static ON_CHANGE: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
    /// último retângulo copiado da barra do Explorer
    static LAST_RECT: Cell<RECT> = const { Cell::new(RECT { left: 0, top: 0, right: 0, bottom: 0 }) };
}

/// Ícones do sistema que o Windows 11 já não mostra na bandeja (volume, rede,
/// energia, central de ações, microfone...): família `7820AE7x/8x-23E3-...`.
/// A ilha mostra o que importa deles.
fn system_icon(g: Option<GUID>) -> bool {
    g.is_some_and(|g| {
        g.data1 & 0xFFFF_FF00 == 0x7820_AE00
            && g.data2 == 0x23E3
            && g.data3 == 0x4229
            && g.data4 == [0x82, 0xC1, 0xE4, 0x1C, 0xB6, 0x7D, 0x5B, 0x9C]
    })
}

// ------------------------------------------------------------ pacote dos apps

/// Campos de um `NOTIFYICONDATA` de 32 bits (os handles vêm em 4 bytes, de
/// app de 32 ou de 64 bits).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Nid {
    pub hwnd: isize,
    pub uid: u32,
    pub flags: u32,
    pub cb: u32,
    pub hicon: isize,
    pub tip: String,
    pub state: u32,
    pub mask: u32,
    pub version: u32,
    pub guid: Option<GUID>,
}

/// Lê o pacote `SHELLTRAYDATA`: (mensagem NIM_*, dados). `None` se não é um.
pub(crate) fn parse_tray(b: &[u8]) -> Option<(u32, Nid)> {
    let u = |off: usize| b.get(off..off + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    if u(0)? != TRAY_SIG {
        return None;
    }
    let nim = u(4)?;
    // NOTIFYICONDATA a partir do byte 8; o que faltar (app antigo) fica zerado
    let n = |off: usize| u(8 + off).unwrap_or(0);
    let tip = b.get(8 + 24..(8 + 24 + 256).min(b.len())).map(wstr).unwrap_or_default();
    let guid = b.get(8 + 936..8 + 952).map(|g| {
        GUID::from_values(
            u32::from_le_bytes([g[0], g[1], g[2], g[3]]),
            u16::from_le_bytes([g[4], g[5]]),
            u16::from_le_bytes([g[6], g[7]]),
            [g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15]],
        )
    });
    Some((
        nim,
        Nid {
            // handles de 32 bits estendem o sinal (como o LongToHandle)
            hwnd: u(8 + 4)? as i32 as isize,
            uid: n(8),
            flags: n(12),
            cb: n(16),
            hicon: n(20) as i32 as isize,
            tip,
            state: n(280),
            mask: n(284),
            version: n(800),
            guid: guid.filter(|g| *g != GUID::zeroed()),
        },
    ))
}

/// UTF-16 até o primeiro zero.
fn wstr(b: &[u8]) -> String {
    let v: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
    String::from_utf16_lossy(&v)
}

/// Aplica a mensagem à tabela e devolve o que responder ao app (`true` =
/// aceito), como uma bandeja recém-criada responderia.
fn apply(nim: u32, d: &Nid) -> (bool, bool) {
    let mut t = TABLE.lock().unwrap();
    let by_guid = d.flags & NIF_GUID != 0 && d.guid.is_some();
    let idx = t.iter().position(|i| if by_guid { i.guid == d.guid } else { i.hwnd == d.hwnd && i.uid == d.uid });
    let update = |i: &mut Icon| {
        if d.flags & NIF_MESSAGE != 0 {
            i.cb = d.cb;
        }
        if d.flags & NIF_ICON != 0 {
            i.free();
            if d.hicon != 0 {
                // cópia nossa: o app pode destruir o dele a qualquer momento
                i.hicon = unsafe { CopyIcon(HICON(d.hicon as *mut _)) }.map(|h| h.0 as isize).unwrap_or(0);
            }
            i.png = None;
        }
        if d.flags & NIF_TIP != 0 {
            i.tip = d.tip.clone();
        }
        if d.flags & NIF_STATE != 0 && d.mask & NIS_HIDDEN != 0 {
            i.hidden = d.state & NIS_HIDDEN != 0;
        }
        if d.flags & NIF_GUID != 0 && d.guid.is_some() {
            i.guid = d.guid;
        }
    };
    let fresh = || Icon {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        hwnd: d.hwnd,
        uid: d.uid,
        guid: None,
        cb: 0,
        hicon: 0,
        tip: String::new(),
        version: 0,
        hidden: false,
        png: None,
    };
    // (resposta, a lista mudou)
    match (nim, idx) {
        (NIM_ADD, Some(i)) => {
            update(&mut t[i]);
            (true, true)
        }
        (NIM_ADD, None) => {
            let mut i = fresh();
            update(&mut i);
            t.push(i);
            (true, true)
        }
        (NIM_MODIFY, Some(i)) => {
            update(&mut t[i]);
            (true, true)
        }
        (NIM_MODIFY, None) => {
            // desconhecido (registrado antes de a dock ligar): já mostra o que
            // veio, mas recusa para o app refazer o ADD completo
            if d.flags & NIF_ICON != 0 {
                let mut i = fresh();
                update(&mut i);
                t.push(i);
                (false, true)
            } else {
                (false, false)
            }
        }
        (NIM_DELETE, Some(i)) => {
            t.remove(i).free();
            (true, true)
        }
        (NIM_SETVERSION, Some(i)) => {
            t[i].version = d.version;
            (d.version <= 4, false)
        }
        (NIM_SETFOCUS, _) => (true, false),
        _ => (false, false),
    }
}

// ------------------------------------------------------------ janela

/// Repassa `WM_COPYDATA` à barra do Explorer. `None` se não há barra ou ela
/// não respondeu em 2 s.
unsafe fn forward(msg: u32, wp: WPARAM, lp: LPARAM) -> Option<isize> {
    let fwd = FWD.load(Ordering::SeqCst);
    if fwd == 0 {
        return None;
    }
    if msg != WM_COPYDATA && !InSendMessage().as_bool() {
        // chegou por PostMessage: segue do mesmo jeito
        let _ = PostMessageW(Some(hwnd(fwd)), msg, wp, lp);
        return Some(0);
    }
    let mut res = 0usize;
    let ok = SendMessageTimeoutW(hwnd(fwd), msg, wp, lp, SMTO_ABORTIFHUNG, 2000, Some(&mut res));
    (ok.0 != 0).then_some(res as isize)
}

fn makelong(lo: i32, hi: i32) -> u32 {
    ((hi as u16 as u32) << 16) | lo as u16 as u32
}

unsafe fn copydata(wp: WPARAM, lp: LPARAM) -> LRESULT {
    let cds = lp.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return LRESULT(0);
    }
    let cds = &*cds;
    let data: &[u8] = if cds.lpData.is_null() || cds.cbData == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize)
    };
    match cds.dwData {
        1 => {
            let ours = parse_tray(data).map(|(nim, d)| {
                let (ok, changed) = apply(nim, &d);
                if changed {
                    notify_soon();
                }
                (nim, ok)
            });
            let fwd = forward(WM_COPYDATA, wp, lp);
            LRESULT(match ours {
                None => fwd.unwrap_or(0),
                Some((NIM_ADD | NIM_MODIFY | NIM_SETVERSION, ok)) => ok as isize,
                Some((_, ok)) => (ok || fwd.unwrap_or(0) != 0) as isize,
            })
        }
        // posição do ícone (`Shell_NotifyIconGetRect`): 1 = canto de cima à
        // esquerda, 2 = de baixo à direita. Todos moram atrás do ⌃.
        3 => {
            let m = data.get(4..8).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
            match (*ANCHOR.lock().unwrap(), m) {
                (Some((l, t, _, _)), Some(1)) => LRESULT(makelong(l, t) as isize),
                (Some((_, _, r, b)), Some(2)) => LRESULT(makelong(r, b) as isize),
                _ => LRESULT(forward(WM_COPYDATA, wp, lp).unwrap_or(0)),
            }
        }
        // AppBars (dwData 0) e o resto: quem responde é o Explorer
        _ => LRESULT(forward(WM_COPYDATA, wp, lp).unwrap_or(0)),
    }
}

/// Avisa a página daqui a pouco (rajadas de mudanças viram um aviso só).
fn notify_soon() {
    let h = HOST.load(Ordering::SeqCst);
    if h != 0 && !NOTIFY_ARMED.swap(true, Ordering::SeqCst) {
        unsafe {
            SetTimer(Some(hwnd(h)), ID_NOTIFY, 150, None);
        }
    }
}

static HOST: AtomicIsize = AtomicIsize::new(0);

/// A cada segundo: segue a barra do Explorer (reinício, retângulo), continua
/// sendo a primeira `Shell_TrayWnd` do `FindWindow` e tira da lista os ícones
/// de apps que morreram sem `NIM_DELETE`.
unsafe fn keep(h: HWND) {
    let mut fwd = FWD.load(Ordering::SeqCst);
    if fwd == 0 || !IsWindow(Some(hwnd(fwd))).as_bool() {
        fwd = taskbar::tray().map(|t| t.0 as isize).unwrap_or(0);
        FWD.store(fwd, Ordering::SeqCst);
    }
    // mesmo retângulo da barra do Explorer (escondida): quem mede a barra
    // pela `Shell_TrayWnd` (menu Iniciar, avisos) continua vendo o mesmo lugar
    let mut r = LAST_RECT.get();
    if fwd != 0 {
        let _ = GetWindowRect(hwnd(fwd), &mut r);
    }
    let first = FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or_default();
    if first != h || r != LAST_RECT.get() {
        LAST_RECT.set(r);
        let _ = SetWindowPos(
            h,
            Some(HWND_TOPMOST),
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
    let gone = {
        let mut t = TABLE.lock().unwrap();
        let before = t.len();
        t.retain_mut(|i| {
            let alive = IsWindow(Some(hwnd(i.hwnd))).as_bool();
            if !alive {
                i.free();
            }
            alive
        });
        t.len() != before
    };
    if gone {
        notify_soon();
    }
}

unsafe extern "system" fn host_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COPYDATA => copydata(wp, lp),
        WM_TIMER => {
            match wp.0 {
                ID_KEEP => keep(h),
                ID_NOTIFY => {
                    let _ = KillTimer(Some(h), ID_NOTIFY);
                    NOTIFY_ARMED.store(false, Ordering::SeqCst);
                    ON_CHANGE.with(|f| {
                        if let Some(f) = f.borrow().as_ref() {
                            f();
                        }
                    });
                }
                ID_REBROADCAST => {
                    let _ = KillTimer(Some(h), ID_REBROADCAST);
                    taskbar::broadcast_taskbar_created();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(h);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::SeqCst) => {
            // a nossa própria difusão também chega aqui: só conta se a barra do
            // Explorer é outra (ele reiniciou). A nova nasce acima da nossa na
            // ordem Z e os apps se registram direto nela; volta ao topo e pede
            // de novo daqui a pouco, para os ícones passarem por aqui
            let now = taskbar::tray().map(|t| t.0 as isize).unwrap_or(0);
            if now != 0 && now != FWD.load(Ordering::SeqCst) {
                FWD.store(now, Ordering::SeqCst);
                keep(h);
                SetTimer(Some(h), ID_REBROADCAST, 1500, None);
            }
            LRESULT(0)
        }
        // pedidos à barra (comandos, mensagens próprias do Explorer): repassa
        WM_COMMAND | WM_USER..=0x7FFF => LRESULT(forward(msg, wp, lp).unwrap_or(0)),
        _ => DefWindowProcW(h, msg, wp, lp),
    }
}

/// A bandeja própria ligada (thread `dock-tray`). Parar devolve os ícones ao
/// Explorer.
pub struct TrayHost {
    hwnd: isize,
    done: mpsc::Receiver<()>,
}

impl TrayHost {
    /// Sobe a janela `Shell_TrayWnd` própria. `on_change` roda na thread dela,
    /// logo depois de a lista de ícones mudar (não pode travar).
    pub fn start(on_change: Box<dyn Fn() + Send>) -> Option<TrayHost> {
        let (ready_tx, ready_rx) = mpsc::sync_channel::<isize>(1);
        let (done_tx, done_rx) = mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("dock-tray".into())
            .spawn(move || {
                host_thread(on_change, ready_tx);
                let _ = done_tx.send(());
            })
            .ok()?;
        let hwnd = ready_rx.recv_timeout(Duration::from_secs(5)).ok().filter(|h| *h != 0)?;
        Some(TrayHost { hwnd, done: done_rx })
    }

    /// Fecha a janela e espera a thread difundir `TaskbarCreated` (os apps
    /// voltam a falar direto com o Explorer, que já tinha tudo).
    pub fn stop(self) {
        unsafe {
            let _ = PostMessageW(Some(hwnd(self.hwnd)), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        let _ = self.done.recv_timeout(Duration::from_secs(3));
    }
}

fn host_thread(on_change: Box<dyn Fn() + Send>, ready: mpsc::SyncSender<isize>) {
    unsafe {
        let hinst = GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default();
        let cls = w!("Shell_TrayWnd");
        // falha só se já existe (dock desligada e ligada de novo): serve a mesma
        RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(host_proc), hInstance: hinst, lpszClassName: cls, ..Default::default() });
        let created = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED.store(created, Ordering::SeqCst);
        // a barra do Explorer, antes de existir a nossa
        FWD.store(taskbar::tray().map(|t| t.0 as isize).unwrap_or(0), Ordering::SeqCst);
        // escondida (sem WS_VISIBLE) e "sempre no topo": a primeira do FindWindow
        let Ok(h) = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, cls, w!(""), WS_POPUP, 0, 0, 0, 0, None, None, Some(hinst), None)
        else {
            let _ = ready.send(0);
            return;
        };
        // RAD como administrador: apps comuns ainda conseguem mandar os ícones
        let _ = ChangeWindowMessageFilterEx(h, WM_COPYDATA, MSGFLT_ALLOW, None);
        let _ = ChangeWindowMessageFilterEx(h, created, MSGFLT_ALLOW, None);
        HOST.store(h.0 as isize, Ordering::SeqCst);
        ON_CHANGE.with(|f| *f.borrow_mut() = Some(on_change));
        keep(h);
        SetTimer(Some(h), ID_KEEP, 1000, None);
        let _ = ready.send(h.0 as isize);
        // os apps registram os ícones de novo, agora aqui
        taskbar::broadcast_taskbar_created();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        HOST.store(0, Ordering::SeqCst);
        NOTIFY_ARMED.store(false, Ordering::SeqCst);
        ON_CHANGE.with(|f| *f.borrow_mut() = None);
        for mut i in TABLE.lock().unwrap().drain(..) {
            i.free();
        }
        *ANCHOR.lock().unwrap() = None;
        // janela destruída: o FindWindow volta a achar o Explorer
        taskbar::broadcast_taskbar_created();
    }
}

// ------------------------------------------------------------ para a página

/// Onde está o botão ⌃ (px físicos da tela), ou `None` se escondido.
pub fn set_tray_anchor(r: Option<(i32, i32, i32, i32)>) {
    *ANCHOR.lock().unwrap() = r;
}

/// Ícones visíveis, na ordem de chegada, com a imagem já em PNG.
pub fn tray_icons() -> Vec<TrayIconOut> {
    let mut t = TABLE.lock().unwrap();
    t.iter_mut()
        .filter(|i| !i.hidden && i.hicon != 0 && !system_icon(i.guid) && unsafe { IsWindow(Some(hwnd(i.hwnd))) }.as_bool())
        .map(|i| {
            if i.png.is_none() {
                i.png = Some(unsafe { hicon_png(HICON(i.hicon as *mut _)) });
            }
            TrayIconOut { id: i.id.to_string(), tip: i.tip.clone(), icon: i.png.clone().flatten() }
        })
        .collect()
}

/// Clique num ícone da janela da bandeja: manda ao app as mensagens que a
/// barra nativa mandaria. `how` = "left" | "right" | "double".
pub fn tray_click(id: &str, how: &str) {
    let Ok(id) = id.parse::<u64>() else { return };
    let it = TABLE.lock().unwrap().iter().find(|i| i.id == id).map(|i| (i.hwnd, i.uid, i.cb, i.version));
    let Some((hw, uid, cb, ver)) = it else { return };
    if cb == 0 {
        return;
    }
    let h = hwnd(hw);
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
        // o menu do app precisa vir para a frente (o RAD está em primeiro plano agora)
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid != 0 {
            let _ = AllowSetForegroundWindow(pid);
        }
    }
    let v4 = ver >= 4;
    let seq: &[u32] = match (how, v4) {
        ("left", true) => &[WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, NIN_SELECT],
        ("left", false) => &[WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP],
        ("right", true) => &[WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_CONTEXTMENU],
        ("right", false) => &[WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP],
        ("double", _) => &[WM_LBUTTONDBLCLK, WM_LBUTTONUP],
        _ => &[],
    };
    for &m in seq {
        // versão 4: wParam = ponto na tela, lParam = (mensagem, id); antes: wParam = id, lParam = mensagem
        let (wp, lp) = if v4 { (makelong(p.x, p.y) as usize, makelong(m as i32, uid as i32) as isize) } else { (uid as usize, m as isize) };
        unsafe {
            let _ = PostMessageW(Some(h), cb, WPARAM(wp), LPARAM(lp));
        }
    }
}

/// HICON -> data URL PNG. Ícone com alfa usa o alfa; sem alfa (antigo), a
/// máscara diz o que é transparente.
unsafe fn hicon_png(h: HICON) -> Option<String> {
    let mut ii = ICONINFO::default();
    GetIconInfo(h, &mut ii).ok()?;
    let out = (|| {
        if ii.hbmColor.is_invalid() {
            return None; // monocromático: raro na bandeja
        }
        let (w, hgt, mut px) = super::win::hbitmap_bgra(ii.hbmColor)?;
        if !px.chunks_exact(4).any(|p| p[3] != 0) {
            let mask = (!ii.hbmMask.is_invalid()).then(|| super::win::hbitmap_bgra(ii.hbmMask)).flatten();
            match mask {
                Some((mw, mh, m)) if mw == w && mh >= hgt => {
                    for (p, q) in px.chunks_exact_mut(4).zip(m.chunks_exact(4)) {
                        p[3] = if q[0] != 0 { 0 } else { 255 };
                    }
                }
                _ => px.chunks_exact_mut(4).for_each(|p| p[3] = 255),
            }
        }
        super::win::bgra_to_rgba(&mut px);
        let png = super::win::encode_png(w, hgt, &px)?;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
    })();
    if !ii.hbmColor.is_invalid() {
        let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
    }
    if !ii.hbmMask.is_invalid() {
        let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{makelong, parse_tray, system_icon, NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, TRAY_SIG};
    use windows::core::GUID;

    /// Pacote como o do Windows 11: cabeçalho, NOTIFYICONDATA de 32 bits e
    /// o caminho do exe no fim (1484 bytes).
    fn pacote(nim: u32, hwnd: u32, uid: u32, flags: u32, tip: &str, guid: Option<GUID>) -> Vec<u8> {
        let mut b = vec![0u8; 1484];
        let put = |b: &mut Vec<u8>, off: usize, v: u32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 0, TRAY_SIG);
        put(&mut b, 4, nim);
        put(&mut b, 8, 956);
        put(&mut b, 12, hwnd);
        put(&mut b, 16, uid);
        put(&mut b, 20, flags);
        put(&mut b, 24, 0x8001);
        put(&mut b, 28, 0x10366);
        for (k, c) in tip.encode_utf16().enumerate() {
            b[32 + k * 2..34 + k * 2].copy_from_slice(&c.to_le_bytes());
        }
        if let Some(g) = guid {
            b[944..948].copy_from_slice(&g.data1.to_le_bytes());
            b[948..950].copy_from_slice(&g.data2.to_le_bytes());
            b[950..952].copy_from_slice(&g.data3.to_le_bytes());
            b[952..960].copy_from_slice(&g.data4);
        }
        put(&mut b, 808, 4);
        b
    }

    #[test]
    fn le_o_pacote_do_windows_11() {
        let b = pacote(0, 0x2034E, 131918, NIF_MESSAGE | NIF_ICON | NIF_TIP, "MSI Afterburner", None);
        let (nim, d) = parse_tray(&b).unwrap();
        assert_eq!(nim, 0);
        assert_eq!(d.hwnd, 0x2034E);
        assert_eq!(d.uid, 131918);
        assert_eq!(d.cb, 0x8001);
        assert_eq!(d.hicon, 0x10366);
        assert_eq!(d.tip, "MSI Afterburner");
        assert_eq!(d.version, 4);
        assert_eq!(d.guid, None);
        // handle com o bit 31 ligado estende o sinal
        let b = pacote(1, 0x8000_0010, 1, 0, "", None);
        assert_eq!(parse_tray(&b).unwrap().1.hwnd, 0x8000_0010u32 as i32 as isize);
    }

    #[test]
    fn guid_e_icones_do_sistema() {
        let vol = GUID::from_u128(0x7820ae73_23e3_4229_82c1_e41cb67d5b9c);
        let b = pacote(0, 0x10280, 100, NIF_ICON | NIF_GUID, "Alto-falantes", Some(vol));
        let d = parse_tray(&b).unwrap().1;
        assert_eq!(d.guid, Some(vol));
        assert!(system_icon(d.guid));
        // Bluetooth (outro GUID do Explorer) aparece
        assert!(!system_icon(Some(GUID::from_u128(0x6da68f06_00f1_4e6e_8158_7f1ffd4f9db9))));
        assert!(!system_icon(None));
    }

    #[test]
    fn pacote_invalido_ou_curto() {
        assert!(parse_tray(&[]).is_none());
        assert!(parse_tray(&[1, 2, 3, 4, 0, 0, 0, 0]).is_none());
        // app antigo (estrutura curta): lê o que veio, o resto fica zerado
        let mut b = pacote(0, 5, 7, NIF_TIP, "abc", None);
        b.truncate(8 + 88);
        let d = parse_tray(&b).unwrap().1;
        assert_eq!((d.hwnd, d.uid, d.tip.as_str(), d.version), (5, 7, "abc", 0));
    }

    #[test]
    fn makelong_com_coordenada_negativa() {
        assert_eq!(makelong(10, 20), (20 << 16) | 10);
        assert_eq!(makelong(-1, 0) & 0xFFFF, 0xFFFF);
    }
}
