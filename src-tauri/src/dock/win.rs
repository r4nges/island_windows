//! Parte Windows da dock: a thread `dock-shell` (janela auxiliar com loop de
//! mensagens), a lista de apps, os ícones e as ações dos botões.
//!
//! A thread `dock-shell` não faz polling: dorme em `MsgWaitForMultipleObjects`
//! e só acorda por
//! - ganchos `SetWinEventHook` (janela mostrada/escondida/destruída/oculta
//!   pelo DWM, primeiro plano, título), filtrados e agrupados por um timer de
//!   80 ms, para uma rajada virar uma leitura só;
//! - avisos do registro (`RegNotifyChangeKeyValue`): tema claro/escuro dos
//!   apps e a lista de fixados da barra (`Taskband\Favorites`);
//! - mensagens: AppBar, `TaskbarCreated` (Explorer reiniciou), fim da sessão,
//!   troca de resolução e os pedidos das outras threads (vidro, reserva).

use super::taskbar::{self, Hider};
use super::ItemOut;
use base64::Engine as _;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::core::{s, w, BOOL, GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, PROPERTYKEY, RECT, SIZE, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_CLOAKED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, GetStockObject,
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLACK_BRUSH, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
};
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_NOTIFY,
    KEY_QUERY_VALUE, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, CreateEventW, GetCurrentProcessId, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, INFINITE, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_B, VK_LWIN, VK_RETURN, VK_S,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::{
    IShellItem2, IShellItemImageFactory, SHAppBarMessage, SHCreateItemFromIDList, SHCreateItemFromParsingName,
    ShellExecuteExW, ABE_BOTTOM, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, ABN_POSCHANGED, ABN_STATECHANGE, APPBARDATA,
    SEE_MASK_IDLIST, SHELLEXECUTEINFOW, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_FILESYSPATH, SIGDN_NORMALDISPLAY,
    SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EnumWindows, FindWindowExW, GetAncestor,
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowTextLengthW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, KillTimer, MsgWaitForMultipleObjects, PeekMessageW,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    SetWindowPos, ShowWindow, ShowWindowAsync, TranslateMessage, CHILDID_SELF, EVENT_OBJECT_CLOAKED,
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED,
    EVENT_SYSTEM_FOREGROUND, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, HTTRANSPARENT, MA_NOACTIVATE, MSG, OBJID_WINDOW,
    PM_REMOVE, QS_ALLINPUT, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNORMAL,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_ENDSESSION,
    WM_MOUSEACTIVATE, WM_NCHITTEST, WM_QUERYENDSESSION, WM_QUIT, WM_TIMER, WNDCLASSW, WS_EX_APPWINDOW,
    WS_CAPTION, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

// ------------------------------------------------------------ utilidades

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hwnd(h: isize) -> HWND {
    HWND(h as *mut _)
}

/// String alocada pelo sistema (CoTaskMem) -> `String`, liberando a memória.
unsafe fn take_pwstr(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s.filter(|s| !s.is_empty())
}

fn class_name(h: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n.min(buf.len())])
}

const fn pkey(fmtid: u128, pid: u32) -> PROPERTYKEY {
    PROPERTYKEY { fmtid: GUID::from_u128(fmtid), pid }
}
/// System.AppUserModel.ID: o "id do app" que a barra usa para agrupar
const PKEY_AUMID: PROPERTYKEY = pkey(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3, 5);
/// System.Link.TargetParsingPath: alvo de um atalho
const PKEY_LINK_TARGET: PROPERTYKEY = pkey(0xB9B4B3FC_2B51_4A42_B5D8_324146AFCF25, 2);
/// System.FileDescription: nome "bonito" de um exe ("Google Chrome")
const PKEY_FILE_DESCRIPTION: PROPERTYKEY = pkey(0x0CEF7D53_FA64_11D1_A203_0000F81FEDEE, 3);

// ------------------------------------------------------------ apps e janelas

/// De onde sai um ícone ou o que abrir: um item do shell pela lista de IDs
/// (fixado na barra) ou pelo nome (caminho, `shell:AppsFolder\<AUMID>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Src {
    Pidl(Vec<u8>),
    Parse(String),
}

/// Um botão da dock: app fixado e/ou com janelas abertas.
#[derive(Clone, Debug)]
pub struct Item {
    pub key: String,
    pub name: String,
    pub pinned: bool,
    /// o que abrir num clique sem janela (só fixados)
    pub launch: Option<Src>,
    /// candidatos a ícone, na ordem
    pub icon: Vec<Src>,
    /// janelas do app, na ordem Z (a de cima primeiro)
    pub wins: Vec<isize>,
}

/// Atalho fixado na barra do usuário.
#[derive(Clone, Debug)]
struct Pin {
    key: String,
    name: String,
    src: Src,
    aumid: Option<String>,
    /// exe alvo em minúsculas
    exe: Option<String>,
    /// caminho do .lnk em minúsculas (para não repetir o da pasta)
    path: Option<String>,
}

/// Janela que a barra nativa mostraria.
#[derive(Clone, Debug)]
struct Win {
    hwnd: isize,
    aumid: Option<String>,
    exe: Option<String>,
}

impl Win {
    /// Chave do grupo: AUMID quando há (é o que a barra usa), senão o exe.
    fn key(&self) -> String {
        match (&self.aumid, &self.exe) {
            (Some(a), _) => format!("app:{}", a.to_lowercase()),
            (None, Some(e)) => format!("exe:{e}"),
            _ => format!("hwnd:{}", self.hwnd),
        }
    }
}

impl Pin {
    /// A janela é deste fixado? Mesmo AUMID; ou mesmo exe quando um dos dois
    /// não tem AUMID (atalho comum de um app que não declara o seu). Dois
    /// AUMIDs diferentes no mesmo exe (perfis do navegador) ficam separados.
    fn matches(&self, w: &Win) -> bool {
        if let (Some(a), Some(b)) = (&self.aumid, &w.aumid) {
            if a.eq_ignore_ascii_case(b) {
                return true;
            }
        }
        match (&self.exe, &w.exe) {
            (Some(a), Some(b)) => a == b && (self.aumid.is_none() || w.aumid.is_none()),
            _ => false,
        }
    }
}

/// Entradas do valor `Taskband\Favorites`: `00 <u32 tamanho> <lista de IDs>`
/// repetido, terminando em `FF`. É a lista de fixados na ordem da barra,
/// incluindo apps da Store (que não têm .lnk na pasta).
pub(crate) fn parse_favorites(b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while b.get(i) == Some(&0) {
        let Some(sz) = b.get(i + 1..i + 5) else { break };
        let n = u32::from_le_bytes([sz[0], sz[1], sz[2], sz[3]]) as usize;
        let start = i + 5;
        let Some(pidl) = start.checked_add(n).and_then(|end| b.get(start..end)) else { break };
        if valid_pidl(pidl) {
            out.push(pidl.to_vec());
        }
        i = start + n;
    }
    out
}

/// Lista de IDs bem formada: itens `<u16 tamanho><dados>` até um tamanho 0.
fn valid_pidl(p: &[u8]) -> bool {
    let mut i = 0;
    loop {
        let Some(cb) = p.get(i..i + 2) else { return false };
        let cb = u16::from_le_bytes([cb[0], cb[1]]) as usize;
        if cb == 0 {
            return i > 0;
        }
        if cb < 2 {
            return false;
        }
        i += cb;
    }
}

unsafe fn shell_item(src: &Src) -> Option<IShellItem2> {
    match src {
        Src::Pidl(b) => {
            // terminador extra: o tamanho gravado já inclui o dele, mas não custa
            let mut buf = b.clone();
            buf.extend_from_slice(&[0, 0]);
            SHCreateItemFromIDList(buf.as_ptr() as *const ITEMIDLIST).ok()
        }
        Src::Parse(p) => {
            let w = wide(p);
            SHCreateItemFromParsingName(PCWSTR(w.as_ptr()), None::<&IBindCtx>).ok()
        }
    }
}

unsafe fn item_string(it: &IShellItem2, key: &PROPERTYKEY) -> Option<String> {
    it.GetString(key).ok().and_then(|p| take_pwstr(p))
}

fn pin_from(src: Src) -> Option<Pin> {
    unsafe {
        let it = shell_item(&src)?;
        let name = it.GetDisplayName(SIGDN_NORMALDISPLAY).ok().and_then(|p| take_pwstr(p))?;
        let parse = it.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING).ok().and_then(|p| take_pwstr(p)).unwrap_or_default();
        let path = it.GetDisplayName(SIGDN_FILESYSPATH).ok().and_then(|p| take_pwstr(p)).map(|p| p.to_lowercase());
        // item da pasta Apps (app da Store): o nome de análise termina no AUMID
        let aumid = item_string(&it, &PKEY_AUMID)
            .or_else(|| parse.rsplit('\\').next().filter(|s| s.contains('!')).map(String::from));
        let exe = item_string(&it, &PKEY_LINK_TARGET)
            .map(|t| t.to_lowercase())
            .filter(|t| t.ends_with(".exe"));
        let id = aumid.clone().map(|a| a.to_lowercase()).or_else(|| exe.clone()).unwrap_or_else(|| parse.to_lowercase());
        Some(Pin { key: format!("pin:{id}"), name, src, aumid, exe, path })
    }
}

fn reg_binary(key: &str, value: &str) -> Option<Vec<u8>> {
    let (k, v) = (wide(key), wide(value));
    let mut size = 0u32;
    unsafe {
        let _ = RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(v.as_ptr()), RRF_RT_REG_BINARY, None, None, Some(&mut size));
        if size == 0 || size > 1 << 20 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_BINARY,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
        .ok()
        .ok()?;
        buf.truncate(size as usize);
        Some(buf)
    }
}

const TASKBAND: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Taskband";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// Fixados da barra do usuário, na ordem dela: a lista do registro e, depois,
/// os .lnk da pasta `User Pinned\TaskBar` que ela não trouxe (formato do
/// registro mudou numa versão futura do Windows, por exemplo).
fn load_pins() -> Vec<Pin> {
    let mut out: Vec<Pin> = reg_binary(TASKBAND, "Favorites")
        .map(|b| parse_favorites(&b).into_iter().filter_map(|p| pin_from(Src::Pidl(p))).collect())
        .unwrap_or_default();
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for f in files {
            let low = f.to_string_lossy().to_lowercase();
            if out.iter().any(|p| p.path.as_deref() == Some(low.as_str())) {
                continue;
            }
            if let Some(p) = pin_from(Src::Parse(f.to_string_lossy().into_owned())) {
                out.push(p);
            }
        }
    }
    let mut seen = HashSet::new();
    out.retain(|p| seen.insert(p.key.clone()));
    out
}

/// Exe e AUMID de pacote (apps da Store e Win32 empacotados) de um processo.
#[derive(Clone, Default)]
struct Proc {
    exe: Option<String>,
    aumid: Option<String>,
}

fn process_info(pid: u32) -> Proc {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return Proc::default() };
        let mut buf = vec![0u16; 1024];
        let mut n = buf.len() as u32;
        let exe = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut n)
            .ok()
            .map(|_| String::from_utf16_lossy(&buf[..n as usize]).to_lowercase());
        let mut len = buf.len() as u32;
        let aumid = (GetApplicationUserModelId(h, &mut len, Some(PWSTR(buf.as_mut_ptr()))).0 == 0)
            .then(|| String::from_utf16_lossy(&buf[..(len as usize).saturating_sub(1)]))
            .filter(|s| !s.is_empty());
        let _ = CloseHandle(h);
        Proc { exe, aumid }
    }
}

/// AUMID declarado na própria janela (Chrome, Edge, apps com perfil próprio).
fn window_aumid(h: HWND) -> Option<String> {
    unsafe {
        let ps: IPropertyStore = SHGetPropertyStoreForWindow(h).ok()?;
        let mut pv = ps.GetValue(&PKEY_AUMID).ok()?;
        let s = PropVariantToStringAlloc(&pv).ok().and_then(|p| take_pwstr(p));
        let _ = PropVariantClear(&mut pv);
        s
    }
}

/// A janela apareceria na barra nativa? Visível, sem dono (ou com
/// `WS_EX_APPWINDOW`), sem `WS_EX_TOOLWINDOW`/`WS_EX_NOACTIVATE`, não
/// escondida pelo DWM (outra área de trabalho virtual, app da Store suspenso)
/// e com título. Devolve o processo dela.
unsafe fn eligible(h: HWND, own_pid: u32) -> Option<u32> {
    if !IsWindowVisible(h).as_bool() {
        return None;
    }
    let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
    let app = ex & WS_EX_APPWINDOW.0 != 0;
    if ex & WS_EX_TOOLWINDOW.0 != 0 {
        return None;
    }
    if !app && (GetWindow(h, GW_OWNER).is_ok_and(|o| !o.is_invalid()) || ex & WS_EX_NOACTIVATE.0 != 0) {
        return None;
    }
    // sobreposições (a própria ilha de outra cópia, widgets): quem sai da barra
    // por ITaskbarList::DeleteTab, como o "skip taskbar" do Tauri, não tem
    // estilo que denuncie; ficam de fora as janelas que deixam o clique passar
    // e as "sempre no topo" sem barra de título
    let style = GetWindowLongW(h, GWL_STYLE) as u32;
    if ex & WS_EX_TRANSPARENT.0 != 0 || (ex & WS_EX_TOPMOST.0 != 0 && style & WS_CAPTION.0 != WS_CAPTION.0) {
        return None;
    }
    let mut cloaked = 0u32;
    let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
    if cloaked != 0 {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(h, Some(&mut pid));
    if pid == own_pid || pid == 0 {
        return None;
    }
    if !app && GetWindowTextLengthW(h) == 0 {
        return None;
    }
    if matches!(class_name(h).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Progman" | "WorkerW") {
        return None;
    }
    Some(pid)
}

unsafe extern "system" fn collect_cb(h: HWND, lp: LPARAM) -> BOOL {
    let v = &mut *(lp.0 as *mut Vec<isize>);
    v.push(h.0 as isize);
    BOOL(1)
}

/// Janelas de primeiro nível na ordem Z (a de cima primeiro).
fn top_windows() -> Vec<isize> {
    let mut v: Vec<isize> = Vec::with_capacity(512);
    unsafe {
        let _ = EnumWindows(Some(collect_cb), LPARAM(&mut v as *mut Vec<isize> as isize));
    }
    v
}

// ------------------------------------------------------------ thread da dock

/// O que a thread da dock compartilha com o resto do app.
pub struct Shared {
    pub items: Mutex<Vec<Item>>,
    pub fg: AtomicIsize,
    pub light: AtomicBool,
    /// vidro: (x, y, largura, altura) em px físicos da tela
    glass: Mutex<Option<(i32, i32, i32, i32)>>,
    /// faixa reservada como AppBar (esq, topo, dir, base), px físicos
    appbar: Mutex<Option<(i32, i32, i32, i32)>>,
    dock: AtomicIsize,
    visible: AtomicBool,
}

const WM_APP_GLASS: u32 = WM_APP + 1;
const WM_APP_APPBAR: u32 = WM_APP + 2;
const WM_APPBAR_CB: u32 = WM_APP + 3;
const ID_REFRESH: usize = 1;

const DIRTY_LIST: u32 = 1;
const DIRTY_PINS: u32 = 2;

// O gancho de eventos é uma função `extern "system"` sem contexto: o que ele
// precisa fica em estáticos (só existe uma dock por processo).
static DIRTY: AtomicU32 = AtomicU32::new(0);
static TIMER_ARMED: AtomicBool = AtomicBool::new(false);
static HELPER: AtomicIsize = AtomicIsize::new(0);
static TRAY: AtomicIsize = AtomicIsize::new(0);
static HIDING: AtomicBool = AtomicBool::new(false);
/// mensagem `TaskbarCreated` (o Explorer a difunde quando a barra renasce)
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// janelas listadas na última leitura (o gancho ignora as outras)
    static TRACKED: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
}

struct State {
    app: AppHandle,
    shared: Arc<Shared>,
    helper: HWND,
    glass: HWND,
    hider: Hider,
    appbar_on: bool,
    hooks: Vec<HWINEVENTHOOK>,
    pins: Vec<Pin>,
    names: HashMap<String, String>,
    procs: HashMap<u32, Proc>,
    /// ordem de chegada dos apps não fixados
    order: Vec<String>,
    last: Vec<ItemOut>,
    on_display: Box<dyn Fn() + Send>,
    own_pid: u32,
}

/// Roda `f` com o estado da thread. `false` se ele já está em uso (uma chamada
/// COM bombeou mensagens no meio de outra): quem chamou tenta de novo depois.
fn with_state(f: impl FnOnce(&mut State)) -> bool {
    STATE.with(|s| match s.try_borrow_mut() {
        Ok(mut g) => {
            if let Some(st) = g.as_mut() {
                f(st);
            }
            true
        }
        Err(_) => false,
    })
}

fn arm_timer() {
    if !TIMER_ARMED.swap(true, Ordering::SeqCst) {
        unsafe {
            SetTimer(Some(hwnd(HELPER.load(Ordering::SeqCst))), ID_REFRESH, 80, None);
        }
    }
}

fn tracked(h: isize) -> bool {
    // ocupado = no meio de uma leitura: na dúvida, conta como nossa
    TRACKED.with(|t| t.try_borrow().map(|t| t.contains(&h)).unwrap_or(true))
}

unsafe extern "system" fn on_event(_: HWINEVENTHOOK, ev: u32, h: HWND, obj: i32, child: i32, _: u32, _: u32) {
    if obj != OBJID_WINDOW.0 || child != CHILDID_SELF as i32 || h.is_invalid() {
        return;
    }
    let hv = h.0 as isize;
    let top = || GetAncestor(h, GA_ROOT) == h;
    let dirty = match ev {
        EVENT_SYSTEM_FOREGROUND => true,
        EVENT_OBJECT_SHOW => {
            // o Explorer mostrou a barra de novo (mouse na borda, Win+T): esconde
            if HIDING.load(Ordering::SeqCst) && hv == TRAY.load(Ordering::SeqCst) {
                let _ = ShowWindowAsync(h, SW_HIDE);
                return;
            }
            top()
        }
        EVENT_OBJECT_HIDE | EVENT_OBJECT_CLOAKED | EVENT_OBJECT_UNCLOAKED => tracked(hv) || top(),
        EVENT_OBJECT_DESTROY => tracked(hv),
        // título de janela já listada não muda a dock; uma sem título pode passar a contar
        EVENT_OBJECT_NAMECHANGE => !tracked(hv) && top() && IsWindowVisible(h).as_bool(),
        _ => false,
    };
    if dirty {
        DIRTY.fetch_or(DIRTY_LIST, Ordering::SeqCst);
        arm_timer();
    }
}

unsafe extern "system" fn helper_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wp.0 == ID_REFRESH => {
            let _ = KillTimer(Some(h), ID_REFRESH);
            TIMER_ARMED.store(false, Ordering::SeqCst);
            if !with_state(|s| s.refresh()) {
                arm_timer();
            }
            LRESULT(0)
        }
        WM_APP_GLASS => {
            if !with_state(|s| s.apply_glass()) {
                let _ = PostMessageW(Some(h), msg, wp, lp);
            }
            LRESULT(0)
        }
        WM_APP_APPBAR => {
            if !with_state(|s| s.appbar_set()) {
                let _ = PostMessageW(Some(h), msg, wp, lp);
            }
            LRESULT(0)
        }
        WM_APPBAR_CB => {
            // outra AppBar mudou ou o usuário trocou o auto-ocultar da barra
            if wp.0 as u32 == ABN_POSCHANGED || wp.0 as u32 == ABN_STATECHANGE {
                let _ = PostMessageW(Some(h), WM_APP_APPBAR, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            with_state(|s| (s.on_display)());
            DefWindowProcW(h, msg, wp, lp)
        }
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            // fim da sessão: o processo morre logo depois (talvez junto com o
            // vigia); restaura já e apaga o arquivo de recuperação
            if wp.0 != 0 {
                with_state(|s| {
                    HIDING.store(false, Ordering::SeqCst);
                    s.hider.restore();
                });
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
            // Explorer reiniciou: a barra nova nasce visível e a AppBar some
            if !with_state(|s| s.explorer_restarted()) {
                let _ = PostMessageW(Some(h), msg, wp, lp);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(h, msg, wp, lp),
    }
}

unsafe extern "system" fn glass_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // o vidro só fica por baixo da dock: o mouse passa direto
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => DefWindowProcW(h, msg, wp, lp),
    }
}

/// Desfoque do que está atrás (o mesmo efeito do "Transparência" do Windows)
/// por `SetWindowCompositionAttribute`, que não é documentada: sem ela a
/// janela fica preta transparente e a dock mostra só a tinta do CSS.
unsafe fn enable_blur(h: HWND) {
    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        color: u32,
        anim: u32,
    }
    #[repr(C)]
    struct CompositionData {
        attrib: u32,
        data: *mut core::ffi::c_void,
        size: usize,
    }
    type SetWca = unsafe extern "system" fn(HWND, *mut CompositionData) -> BOOL;
    let Ok(user32) = GetModuleHandleW(w!("user32.dll")) else { return };
    let Some(f) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else { return };
    let f: SetWca = std::mem::transmute(f);
    // 3 = ACCENT_ENABLE_BLURBEHIND (desfoque puro; a tinta vem da página)
    let mut pol = AccentPolicy { state: 3, flags: 0, color: 0, anim: 0 };
    // 19 = WCA_ACCENT_POLICY
    let mut d = CompositionData { attrib: 19, data: &mut pol as *mut _ as *mut _, size: std::mem::size_of::<AccentPolicy>() };
    let _ = f(h, &mut d);
}

/// Lê `AppsUseLightTheme` (tema dos apps: claro = 1). Sem o valor, claro (padrão do Windows).
pub fn apps_light() -> bool {
    let (k, v) = (wide(PERSONALIZE), wide("AppsUseLightTheme"));
    let mut val = 1u32;
    let mut size = 4u32;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut val as *mut u32 as *mut _),
            Some(&mut size),
        );
    }
    val != 0
}

/// Chave do registro assinada por evento (`RegNotifyChangeKeyValue`).
struct RegWatch {
    key: HKEY,
    event: HANDLE,
}

impl RegWatch {
    fn new(path: &str) -> Option<Self> {
        let p = wide(path);
        let mut key = HKEY::default();
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), None, KEY_NOTIFY | KEY_QUERY_VALUE, &mut key).is_err() {
                return None;
            }
            let Ok(event) = CreateEventW(None, false, false, None) else {
                let _ = RegCloseKey(key);
                return None;
            };
            let w = Self { key, event };
            w.arm();
            Some(w)
        }
    }

    /// Assina (de novo) a próxima mudança de valor.
    fn arm(&self) {
        unsafe {
            let _ = RegNotifyChangeKeyValue(self.key, false, REG_NOTIFY_CHANGE_LAST_SET, Some(self.event), true);
        }
    }
}

impl Drop for RegWatch {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.key);
            let _ = CloseHandle(self.event);
        }
    }
}

impl State {
    /// Lê janelas e fixados de novo, agrupa e emite `dock:items` se mudou.
    fn refresh(&mut self) {
        let d = DIRTY.swap(0, Ordering::SeqCst);
        if d & DIRTY_PINS != 0 {
            self.pins = load_pins();
        }
        let fg = unsafe { GetForegroundWindow() }.0 as isize;
        self.shared.fg.store(fg, Ordering::SeqCst);

        let mut wins: Vec<Win> = Vec::new();
        let mut procs: HashMap<u32, Proc> = HashMap::new();
        for h in top_windows() {
            let Some(pid) = (unsafe { eligible(hwnd(h), self.own_pid) }) else { continue };
            let p = procs
                .entry(pid)
                .or_insert_with(|| self.procs.get(&pid).cloned().unwrap_or_else(|| process_info(pid)))
                .clone();
            let mut aumid = window_aumid(hwnd(h)).or(p.aumid);
            // app da Store: a moldura é do ApplicationFrameHost; o app é a CoreWindow filha
            if aumid.is_none() && class_name(hwnd(h)) == "ApplicationFrameWindow" {
                aumid = unsafe { FindWindowExW(Some(hwnd(h)), None, w!("Windows.UI.Core.CoreWindow"), None) }
                    .ok()
                    .and_then(|c| {
                        let mut cpid = 0u32;
                        unsafe { GetWindowThreadProcessId(c, Some(&mut cpid)) };
                        process_info(cpid).aumid
                    });
            }
            // pastas do Explorer não declaram AUMID, mas a barra as agrupa com o
            // atalho fixado "Explorador de Arquivos" por este id
            if aumid.is_none() && p.exe.as_deref().is_some_and(|e| e.ends_with(r"\explorer.exe")) {
                aumid = Some("Microsoft.Windows.Explorer".into());
            }
            wins.push(Win { hwnd: h, aumid, exe: p.exe });
        }
        self.procs = procs;
        TRACKED.with(|t| *t.borrow_mut() = wins.iter().map(|w| w.hwnd).collect());

        let items = self.group(wins);
        let out: Vec<ItemOut> = items
            .iter()
            .map(|it| ItemOut {
                key: it.key.clone(),
                name: it.name.clone(),
                pinned: it.pinned,
                open: it.wins.len(),
                focus: it.wins.contains(&fg),
            })
            .collect();
        *self.shared.items.lock().unwrap() = items;
        if out != self.last {
            let _ = self.app.emit_to("dock", "dock:items", &out);
            self.last = out;
        }
    }

    fn group(&mut self, wins: Vec<Win>) -> Vec<Item> {
        let mut items: Vec<Item> = self
            .pins
            .iter()
            .map(|p| Item {
                key: p.key.clone(),
                name: p.name.clone(),
                pinned: true,
                launch: Some(p.src.clone()),
                icon: vec![p.src.clone()],
                wins: Vec::new(),
            })
            .collect();
        let mut extra: HashMap<String, Item> = HashMap::new();
        let mut seen: Vec<String> = Vec::new();
        for w in wins {
            if let Some(i) = self.pins.iter().position(|p| p.matches(&w)) {
                items[i].wins.push(w.hwnd);
                continue;
            }
            let key = w.key();
            if !extra.contains_key(&key) {
                let mut icon = Vec::new();
                if let Some(a) = &w.aumid {
                    icon.push(Src::Parse(format!(r"shell:AppsFolder\{a}")));
                }
                if let Some(e) = &w.exe {
                    icon.push(Src::Parse(e.clone()));
                }
                let name = match self.names.get(&key) {
                    Some(n) => n.clone(),
                    None => {
                        let n = app_name(&w);
                        self.names.insert(key.clone(), n.clone());
                        n
                    }
                };
                extra.insert(key.clone(), Item { key: key.clone(), name, pinned: false, launch: None, icon, wins: Vec::new() });
                seen.push(key.clone());
            }
            extra.get_mut(&key).unwrap().wins.push(w.hwnd);
        }
        // quem já estava na dock mantém o lugar; os novos entram no fim
        self.order.retain(|k| extra.contains_key(k));
        for k in seen {
            if !self.order.contains(&k) {
                self.order.push(k);
            }
        }
        items.extend(self.order.iter().filter_map(|k| extra.remove(k)));
        items
    }

    fn apply_glass(&mut self) {
        let g = *self.shared.glass.lock().unwrap();
        let visible = self.shared.visible.load(Ordering::SeqCst);
        unsafe {
            match g {
                Some((x, y, w, h)) if visible && w > 0 && h > 0 => {
                    // logo abaixo da dock na ordem Z (as duas são "sempre no topo")
                    let dock = self.shared.dock.load(Ordering::SeqCst);
                    let after = if dock != 0 { hwnd(dock) } else { HWND(-1isize as *mut _) };
                    let _ = SetWindowPos(self.glass, Some(after), x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                }
                _ => {
                    let _ = ShowWindow(self.glass, SW_HIDE);
                }
            }
        }
    }

    /// Registra/atualiza a faixa reservada no pé da tela: janelas maximizadas
    /// param acima da dock, como paravam acima da barra nativa.
    fn appbar_set(&mut self) {
        let want = *self.shared.appbar.lock().unwrap();
        let mut d = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.helper,
            uCallbackMessage: WM_APPBAR_CB,
            ..Default::default()
        };
        unsafe {
            let Some((l, t, r, b)) = want else {
                if self.appbar_on {
                    SHAppBarMessage(ABM_REMOVE, &mut d);
                    self.appbar_on = false;
                }
                return;
            };
            // a barra nativa escondida ainda reserva a faixa dela: pede só o resto
            let need = (b - t) - taskbar::bottom_reserve();
            if need <= 0 {
                if self.appbar_on {
                    SHAppBarMessage(ABM_REMOVE, &mut d);
                    self.appbar_on = false;
                }
                return;
            }
            if !self.appbar_on {
                self.appbar_on = SHAppBarMessage(ABM_NEW, &mut d) != 0;
            }
            d.uEdge = ABE_BOTTOM;
            d.rc = RECT { left: l, top: b - need, right: r, bottom: b };
            SHAppBarMessage(ABM_QUERYPOS, &mut d);
            // o sistema sobe a faixa para cima da barra nativa: mantém a altura
            d.rc.top = d.rc.bottom - need;
            SHAppBarMessage(ABM_SETPOS, &mut d);
        }
    }

    fn explorer_restarted(&mut self) {
        let tray = taskbar::tray().map(|t| t.0 as isize).unwrap_or(0);
        TRAY.store(tray, Ordering::SeqCst);
        if self.hider.hidden {
            self.hider.hide();
        }
        // o Explorer novo não conhece a AppBar antiga
        self.appbar_on = false;
        self.appbar_set();
        DIRTY.fetch_or(DIRTY_LIST | DIRTY_PINS, Ordering::SeqCst);
        arm_timer();
    }

    fn theme_changed(&mut self) {
        let light = apps_light();
        if self.shared.light.swap(light, Ordering::SeqCst) != light {
            let _ = self.app.emit_to("dock", "dock:theme", light);
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            for h in self.hooks.drain(..) {
                let _ = UnhookWinEvent(h);
            }
            *self.shared.appbar.lock().unwrap() = None;
            self.appbar_set();
            HIDING.store(false, Ordering::SeqCst);
            self.hider.restore();
            let _ = DestroyWindow(self.glass);
        }
    }
}

/// Nome de um app não fixado: o da pasta Apps (pelo AUMID), senão a
/// descrição do exe ("Microsoft Edge"), senão o nome do arquivo.
fn app_name(w: &Win) -> String {
    unsafe {
        if let Some(a) = &w.aumid {
            if let Some(n) = shell_item(&Src::Parse(format!(r"shell:AppsFolder\{a}")))
                .and_then(|it| it.GetDisplayName(SIGDN_NORMALDISPLAY).ok())
                .and_then(|p| take_pwstr(p))
            {
                return n;
            }
        }
        if let Some(e) = &w.exe {
            if let Some(n) = shell_item(&Src::Parse(e.clone())).and_then(|it| item_string(&it, &PKEY_FILE_DESCRIPTION)) {
                return n;
            }
            let stem = std::path::Path::new(e).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let mut c = stem.chars();
            return c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or(stem);
        }
    }
    "App".into()
}

/// Ponta da dock nas outras threads. Parar (`stop`) restaura a barra nativa.
pub struct Engine {
    pub h: Handle,
    done: mpsc::Receiver<()>,
    marker: PathBuf,
}

/// Pedidos à thread da dock; clonável para usar sem segurar travas.
#[derive(Clone)]
pub struct Handle {
    helper: isize,
    shared: Arc<Shared>,
}

impl Engine {
    /// Sobe a thread `dock-shell`: esconde a barra nativa, reserva a faixa
    /// `appbar`, assina os eventos e lê a primeira lista. `None` se a janela
    /// auxiliar não pôde ser criada (aí nada foi alterado).
    pub fn start(app: AppHandle, marker: PathBuf, appbar: (i32, i32, i32, i32), on_display: Box<dyn Fn() + Send>) -> Option<Engine> {
        let shared = Arc::new(Shared {
            items: Mutex::new(Vec::new()),
            fg: AtomicIsize::new(0),
            light: AtomicBool::new(apps_light()),
            glass: Mutex::new(None),
            appbar: Mutex::new(Some(appbar)),
            dock: AtomicIsize::new(0),
            visible: AtomicBool::new(true),
        });
        let (ready_tx, ready_rx) = mpsc::sync_channel::<isize>(1);
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let sh = shared.clone();
        let mk = marker.clone();
        std::thread::Builder::new()
            .name("dock-shell".into())
            .spawn(move || {
                shell_thread(app, sh, mk, on_display, ready_tx);
                let _ = done_tx.send(());
            })
            .ok()?;
        let helper = ready_rx.recv_timeout(Duration::from_secs(5)).ok().filter(|h| *h != 0)?;
        Some(Engine { h: Handle { helper, shared }, done: done_rx, marker })
    }

    /// Para a thread e espera ela restaurar a barra nativa. Se ela não
    /// responder em 3 s, restaura daqui mesmo (o vigia também faria).
    pub fn stop(self) {
        self.h.post(WM_CLOSE);
        if self.done.recv_timeout(Duration::from_secs(3)).is_err() {
            taskbar::restore_from(&self.marker);
        }
    }
}

impl Handle {
    fn post(&self, msg: u32) {
        unsafe {
            let _ = PostMessageW(Some(hwnd(self.helper)), msg, WPARAM(0), LPARAM(0));
        }
    }

    pub fn set_glass(&self, g: Option<(i32, i32, i32, i32)>) {
        *self.shared.glass.lock().unwrap() = g;
        self.post(WM_APP_GLASS);
    }

    pub fn set_visible(&self, v: bool) {
        self.shared.visible.store(v, Ordering::SeqCst);
        self.post(WM_APP_GLASS);
    }

    pub fn set_dock(&self, h: isize) {
        self.shared.dock.store(h, Ordering::SeqCst);
        self.post(WM_APP_GLASS);
    }

    pub fn set_appbar(&self, r: (i32, i32, i32, i32)) {
        *self.shared.appbar.lock().unwrap() = Some(r);
        self.post(WM_APP_APPBAR);
    }

    pub fn items(&self) -> Vec<ItemOut> {
        let fg = self.shared.fg.load(Ordering::SeqCst);
        self.shared
            .items
            .lock()
            .unwrap()
            .iter()
            .map(|it| ItemOut { key: it.key.clone(), name: it.name.clone(), pinned: it.pinned, open: it.wins.len(), focus: it.wins.contains(&fg) })
            .collect()
    }

    pub fn light(&self) -> bool {
        self.shared.light.load(Ordering::SeqCst)
    }

    pub fn icon_srcs(&self, key: &str) -> Option<Vec<Src>> {
        self.shared.items.lock().unwrap().iter().find(|i| i.key == key).map(|i| i.icon.clone())
    }

    /// Clique num app (ver `click`).
    pub fn click(&self, key: &str) {
        let it = self.shared.items.lock().unwrap().iter().find(|i| i.key == key).cloned();
        if let Some(it) = it {
            click(&it);
        }
    }
}

fn shell_thread(
    app: AppHandle,
    shared: Arc<Shared>,
    marker: PathBuf,
    on_display: Box<dyn Fn() + Send>,
    ready: mpsc::SyncSender<isize>,
) {
    unsafe {
        // STA: a thread tem loop de mensagens e conversa com o shell
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let hinst = GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default();
        let helper_cls = w!("IslandDockHelper");
        let glass_cls = w!("IslandDockGlass");
        RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(helper_proc), hInstance: hinst, lpszClassName: helper_cls, ..Default::default() });
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(glass_proc),
            hInstance: hinst,
            lpszClassName: glass_cls,
            // preto = transparente sob o desfoque
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ..Default::default()
        });
        // janela de primeiro nível escondida (não "message-only"): só assim
        // recebe as mensagens de difusão (TaskbarCreated, fim da sessão)
        let Ok(helper) = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            helper_cls,
            w!("RAD Dock"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst),
            None,
        ) else {
            let _ = ready.send(0);
            return;
        };
        let glass = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            glass_cls,
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst),
            None,
        )
        .unwrap_or_default();
        if !glass.is_invalid() {
            enable_blur(glass);
            // cantos arredondados nativos (o desfoque não respeita SetWindowRgn)
            let pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(glass, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, 4);
        }
        HELPER.store(helper.0 as isize, Ordering::SeqCst);
        TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::SeqCst);
        TRAY.store(taskbar::tray().map(|t| t.0 as isize).unwrap_or(0), Ordering::SeqCst);

        let mut hider = Hider::new(marker);
        let hidden = hider.hide();
        HIDING.store(hidden, Ordering::SeqCst);

        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let hooks: Vec<HWINEVENTHOOK> = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
            (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
        ]
        .iter()
        .map(|&(a, b)| SetWinEventHook(a, b, None, Some(on_event), 0, 0, flags))
        .filter(|h| !h.is_invalid())
        .collect();

        let state = State {
            app,
            shared,
            helper,
            glass,
            hider,
            appbar_on: false,
            hooks,
            pins: Vec::new(),
            names: HashMap::new(),
            procs: HashMap::new(),
            order: Vec::new(),
            last: Vec::new(),
            on_display,
            own_pid: GetCurrentProcessId(),
        };
        STATE.with(|s| *s.borrow_mut() = Some(state));
        with_state(|s| s.appbar_set());
        DIRTY.store(DIRTY_LIST | DIRTY_PINS, Ordering::SeqCst);
        with_state(|s| s.refresh());
        let _ = ready.send(helper.0 as isize);

        let theme = RegWatch::new(PERSONALIZE);
        let band = RegWatch::new(TASKBAND);
        let handles: Vec<HANDLE> = [&theme, &band].iter().filter_map(|w| w.as_ref().map(|w| w.event)).collect();
        let mut msg = MSG::default();
        'outer: loop {
            let r = MsgWaitForMultipleObjects(Some(&handles), false, INFINITE, QS_ALLINPUT);
            let i = r.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
            if i < handles.len() {
                let ev = handles[i];
                if theme.as_ref().is_some_and(|w| w.event == ev) {
                    theme.as_ref().unwrap().arm();
                    with_state(|s| s.theme_changed());
                } else if let Some(b) = band.as_ref().filter(|w| w.event == ev) {
                    b.arm();
                    DIRTY.fetch_or(DIRTY_PINS, Ordering::SeqCst);
                    arm_timer();
                }
                continue;
            }
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'outer;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        drop(theme);
        drop(band);
        let st = STATE.with(|s| s.borrow_mut().take());
        if let Some(mut st) = st {
            st.cleanup();
        }
        HELPER.store(0, Ordering::SeqCst);
        TRACKED.with(|t| t.borrow_mut().clear());
        CoUninitialize();
    }
}

// ------------------------------------------------------------ ações

fn send_keys(keys: &[(VIRTUAL_KEY, bool)]) {
    let inputs: Vec<INPUT> = keys
        .iter()
        .map(|&(vk, up)| {
            let mut flags = KEYBD_EVENT_FLAGS(0);
            if up {
                flags |= KEYEVENTF_KEYUP;
            }
            if vk == VK_LWIN {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
            }
        })
        .collect();
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Abre um item do shell numa thread própria (o ShellExecute quer COM em STA).
fn shell_open(src: Src) {
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        match &src {
            Src::Pidl(b) => {
                let mut buf = b.clone();
                buf.extend_from_slice(&[0, 0]);
                let mut info = SHELLEXECUTEINFOW {
                    cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                    fMask: SEE_MASK_IDLIST,
                    lpIDList: buf.as_mut_ptr() as *mut _,
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };
                let _ = ShellExecuteExW(&mut info);
            }
            Src::Parse(p) => {
                let f = wide(p);
                let mut info = SHELLEXECUTEINFOW {
                    cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                    lpFile: PCWSTR(f.as_ptr()),
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };
                let _ = ShellExecuteExW(&mut info);
            }
        }
        CoUninitialize();
    });
}

/// Iniciar (Win), Pesquisar (Win+S), ícones ocultos (Win+B e Enter, que
/// funciona com a barra nativa escondida) e Lixeira.
pub fn action(kind: &str) {
    match kind {
        "start" => send_keys(&[(VK_LWIN, false), (VK_LWIN, true)]),
        "search" => send_keys(&[(VK_LWIN, false), (VK_S, false), (VK_S, true), (VK_LWIN, true)]),
        "tray" => {
            std::thread::spawn(|| {
                send_keys(&[(VK_LWIN, false), (VK_B, false), (VK_B, true), (VK_LWIN, true)]);
                // o foco precisa chegar ao botão ⌃ da barra antes do Enter
                std::thread::sleep(Duration::from_millis(200));
                send_keys(&[(VK_RETURN, false), (VK_RETURN, true)]);
            });
        }
        "trash" => shell_open(Src::Parse("shell:RecycleBinFolder".into())),
        _ => {}
    }
}

pub fn trash_src() -> Src {
    Src::Parse("::{645FF040-5081-101B-9F08-00AA002F954E}".into())
}

/// Põe a janela em primeiro plano. O clique na dock chega ao processo do
/// WebView2, não ao Island, então a trava de primeiro plano do Windows recusa
/// o `SetForegroundWindow` simples. Dois recursos conhecidos, nesta ordem:
/// ligar a fila de entrada à da janela em foco (`AttachThreadInput`) e, se
/// ainda não deu, uma tecla sem função (0xE8, não atribuída) para o Island
/// passar a ser "quem recebeu a última entrada".
fn activate(h: HWND) {
    unsafe {
        if IsIconic(h).as_bool() {
            // assíncrono (a janela é de outro processo, que pode estar travado);
            // espera até 150 ms ela sair do minimizado antes de ativá-la
            let _ = ShowWindowAsync(h, SW_RESTORE);
            for _ in 0..15 {
                if !IsIconic(h).as_bool() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if SetForegroundWindow(h).as_bool() && GetForegroundWindow() == h {
            return;
        }
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        if fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool() {
            let _ = BringWindowToTop(h);
            let _ = SetForegroundWindow(h);
            let _ = AttachThreadInput(me, fg_thread, false);
        }
        if GetForegroundWindow() != h {
            send_keys(&[(VIRTUAL_KEY(0xE8), false), (VIRTUAL_KEY(0xE8), true)]);
            let _ = SetForegroundWindow(h);
        }
    }
}

/// Clique num app: sem janela abre; com uma, foca (ou minimiza se já está em
/// foco); com várias, alterna entre elas: a mais recente primeiro e, com uma
/// delas em foco, a que está há mais tempo sem uso (na ordem Z atual).
fn click(it: &Item) {
    if it.wins.is_empty() {
        if let Some(src) = &it.launch {
            shell_open(src.clone());
        }
        return;
    }
    let fg = unsafe { GetForegroundWindow() }.0 as isize;
    if it.wins.len() == 1 {
        let h = hwnd(it.wins[0]);
        if it.wins[0] == fg && !unsafe { IsIconic(h) }.as_bool() {
            unsafe {
                let _ = ShowWindowAsync(h, SW_MINIMIZE);
            }
        } else {
            activate(h);
        }
        return;
    }
    let z = top_windows();
    let mut wins = it.wins.clone();
    wins.sort_by_key(|h| z.iter().position(|x| x == h).unwrap_or(usize::MAX));
    let target = if wins.contains(&fg) { *wins.last().unwrap() } else { wins[0] };
    activate(hwnd(target));
}

pub fn cursor() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

// ------------------------------------------------------------ ícones

type IconReq = (Vec<Src>, mpsc::SyncSender<Option<String>>);
static ICONS: OnceLock<Mutex<mpsc::Sender<IconReq>>> = OnceLock::new();

/// Lado do ícone pedido ao shell, em px: cobre o maior ícone (48 px) com a
/// ampliação forte numa tela a 125%.
const ICON_PX: i32 = 128;

/// Ícone do primeiro candidato que o shell entregar, como data URL PNG.
/// Uma thread própria (STA) extrai e guarda em cache por fonte.
pub fn icon(srcs: Vec<Src>) -> Option<String> {
    let tx = ICONS
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<IconReq>();
            std::thread::Builder::new()
                .name("dock-icons".into())
                .spawn(move || {
                    unsafe {
                        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                    }
                    let mut cache: HashMap<Vec<Src>, Option<String>> = HashMap::new();
                    for (srcs, reply) in rx {
                        let v = cache
                            .entry(srcs.clone())
                            .or_insert_with(|| srcs.iter().find_map(|s| unsafe { image_png(s, ICON_PX) }))
                            .clone();
                        let _ = reply.send(v);
                    }
                })
                .expect("thread de ícones");
            Mutex::new(tx)
        })
        .lock()
        .unwrap()
        .clone();
    let (rtx, rrx) = mpsc::sync_channel(1);
    tx.send((srcs, rtx)).ok()?;
    rrx.recv_timeout(Duration::from_secs(10)).ok().flatten()
}

unsafe fn image_png(src: &Src, px: i32) -> Option<String> {
    use windows::core::Interface;
    let it = shell_item(src)?;
    let f: IShellItemImageFactory = it.cast().ok()?;
    let hbm = f.GetImage(SIZE { cx: px, cy: px }, SIIGBF_ICONONLY).ok()?;
    let out = hbitmap_rgba(hbm);
    let _ = DeleteObject(HGDIOBJ(hbm.0));
    let (w, h, rgba) = out?;
    let png = encode_png(w, h, &rgba)?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
}

/// Pixels RGBA (alfa direto) de um HBITMAP de 32 bits do shell.
unsafe fn hbitmap_rgba(hbm: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut bm = BITMAP::default();
    if GetObjectW(HGDIOBJ(hbm.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 {
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
    if w <= 0 || h <= 0 || w > 1024 || h > 1024 {
        return None;
    }
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // de cima para baixo
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w * h * 4) as usize];
    let dc = CreateCompatibleDC(None);
    let n = GetDIBits(dc, hbm, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
    let _ = DeleteDC(dc);
    if n == 0 {
        return None;
    }
    bgra_to_rgba(&mut px);
    Some((w as u32, h as u32, px))
}

/// BGRA do shell -> RGBA com alfa direto. Sem alfa nenhum (ícone antigo),
/// opaco; com cores todas <= alfa, o bitmap veio pré-multiplicado e é desfeito.
pub(crate) fn bgra_to_rgba(px: &mut [u8]) {
    let any_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    let premul = any_alpha && px.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
    for p in px.chunks_exact_mut(4) {
        let (b, g, r, mut a) = (p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32);
        if !any_alpha {
            a = 255;
        }
        let un = |c: u32| if premul && a > 0 && a < 255 { (c * 255 + a / 2) / a } else { c }.min(255) as u8;
        p[0] = un(r);
        p[1] = un(g);
        p[2] = un(b);
        p[3] = a as u8;
    }
}

fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(rgba).ok()?;
        wr.finish().ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{bgra_to_rgba, parse_favorites, valid_pidl};

    fn entry(pidl: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8];
        v.extend_from_slice(&(pidl.len() as u32).to_le_bytes());
        v.extend_from_slice(pidl);
        v
    }

    #[test]
    fn favoritos_em_ordem() {
        // dois itens: [cb=4 'a' 'b'][0 0] e [cb=3 'c'][0 0]
        let a = [4u8, 0, b'a', b'b', 0, 0];
        let b = [3u8, 0, b'c', 0, 0];
        let mut blob = entry(&a);
        blob.extend(entry(&b));
        blob.push(0xFF);
        assert_eq!(parse_favorites(&blob), vec![a.to_vec(), b.to_vec()]);
    }

    #[test]
    fn favoritos_corrompidos_nao_quebram() {
        // tamanho além do fim: para sem ler fora
        let mut blob = vec![0u8];
        blob.extend_from_slice(&1000u32.to_le_bytes());
        blob.extend_from_slice(&[4, 0, 1, 2, 0, 0]);
        assert!(parse_favorites(&blob).is_empty());
        assert!(parse_favorites(&[]).is_empty());
        assert!(parse_favorites(&[0xFF]).is_empty());
        // lista de IDs sem terminador é descartada, a seguinte continua valendo
        let ok = [3u8, 0, 9, 0, 0];
        let mut blob = entry(&[5, 0, 1, 2, 3]);
        blob.extend(entry(&ok));
        assert_eq!(parse_favorites(&blob), vec![ok.to_vec()]);
        assert!(!valid_pidl(&[0, 0]));
        assert!(!valid_pidl(&[1, 0, 0, 0]));
    }

    #[test]
    fn bitmap_premultiplicado_vira_alfa_direto() {
        // meio transparente pré-multiplicado: BGRA (0,0,100,128) -> RGBA (199,0,0,128)
        let mut px = vec![0, 0, 100, 128, 0, 0, 0, 0];
        bgra_to_rgba(&mut px);
        assert_eq!(&px[..4], &[199, 0, 0, 128]);
        // sem alfa nenhum: opaco, cores mantidas
        let mut px = vec![10, 20, 30, 0];
        bgra_to_rgba(&mut px);
        assert_eq!(px, vec![30, 20, 10, 255]);
        // alfa direto (cor > alfa): não mexe nas cores
        let mut px = vec![200, 0, 0, 100];
        bgra_to_rgba(&mut px);
        assert_eq!(px, vec![0, 0, 200, 100]);
    }
}
