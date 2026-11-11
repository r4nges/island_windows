//! Esconde e restaura a barra de tarefas nativa do Windows.
//!
//! Como esconde: só `ShowWindow(SW_HIDE)` na janela `Shell_TrayWnd`. Nenhuma
//! configuração do Windows muda (nada de auto-ocultar nem registro), então
//! reiniciar o Explorer sempre traz a barra de volta. Escondida, a barra
//! continua reservando a faixa dela no pé da tela; a dock reserva como AppBar
//! só o que falta acima disso (ver `bottom_reserve`).
//!
//! O auto-ocultar foi testado e descartado: ele devolve a faixa, mas é uma
//! mudança persistente e o Explorer "desesconde" a barra sempre que o mouse
//! encosta na borda (as dicas dos botões dela apareciam por cima da dock).
//!
//! Garantia de volta (o release usa `panic = "abort"`, então num travamento
//! nenhum código do Island roda):
//! 1. antes de esconder, grava `taskbar-restore.json` com o estado da barra;
//! 2. um processo vigia (o próprio exe com `--taskbar-watchdog <pid> <arquivo>`)
//!    fica bloqueado esperando o Island terminar (zero CPU) e, se o arquivo
//!    ainda existir, mostra a barra e o apaga;
//! 3. ao iniciar, o Island restaura o que o arquivo disser (vigia morto junto,
//!    ex.: `taskkill /im island.exe /f`, que pega os dois pelo nome);
//! 4. desligar o módulo, sair e o fim da sessão do Windows restauram e apagam
//!    o arquivo, e encerram o vigia;
//! 5. no pior caso, reiniciar o Explorer (ou o PC) mostra a barra.
//!
//! Só a barra principal é escondida: as `Shell_SecondaryTrayWnd` dos outros
//! monitores ficam (a dock só existe no principal).
//!
//! Com a dock ligada o RAD também tem uma janela `Shell_TrayWnd` (a bandeja
//! própria, `tray.rs`): `tray()` pula as janelas do próprio processo.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
use windows::Win32::UI::Shell::{
    SHAllocShared, SHAppBarMessage, SHFreeShared, SHLockShared, SHUnlockShared, ABE_BOTTOM, ABM_GETSTATE,
    ABM_GETTASKBARPOS, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowThreadProcessId, RegisterWindowMessageW, SendMessageTimeoutW, SendNotifyMessageW,
    ShowWindow, ShowWindowAsync, HWND_BROADCAST, SMTO_ABORTIFHUNG, SW_HIDE, SW_SHOWNA, WM_COPYDATA,
};

/// Argumento que transforma o exe no processo vigia.
pub const WATCHDOG_ARG: &str = "--taskbar-watchdog";

/// Estado da barra ao esconder. O Island não muda o auto-ocultar; o estado
/// fica gravado para a restauração conferir (e desfazer, se algo o mudou).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Marker {
    /// retorno do `ABM_GETSTATE` (bit 1 = auto-ocultar)
    state: u32,
}

/// Janela da barra principal do Explorer (muda quando ele reinicia). A
/// `Shell_TrayWnd` da bandeja própria do RAD fica de fora.
pub fn tray() -> Option<HWND> {
    let me = unsafe { GetCurrentProcessId() };
    let mut after: Option<HWND> = None;
    loop {
        let h = unsafe { FindWindowExW(None, after, w!("Shell_TrayWnd"), None) }.ok()?;
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        if pid != me {
            return Some(h);
        }
        after = Some(h);
    }
}

/// Difunde `TaskbarCreated`: todo app com ícone na bandeja o registra de novo
/// na `Shell_TrayWnd` que o `FindWindow` achar (a do RAD ou a do Explorer).
/// Assíncrono, não espera ninguém.
pub fn broadcast_taskbar_created() {
    unsafe {
        let m = RegisterWindowMessageW(w!("TaskbarCreated"));
        if m != 0 {
            let _ = SendNotifyMessageW(HWND_BROADCAST, m, WPARAM(0), LPARAM(0));
        }
    }
}

/// `SHAppBarMessage` para as chamadas do próprio RAD.
///
/// Com a bandeja própria ligada, o `SHAppBarMessage` acha a `Shell_TrayWnd`
/// do RAD. Para outro processo isso não muda nada: o shell32 cria o bloco da
/// resposta no processo da bandeja e o Explorer o abre pelo id do processo,
/// que vem no pacote repassado. Mas, quando quem chama é o próprio processo da
/// bandeja, o shell32 manda um ponteiro em vez do bloco, e o Explorer não tem
/// como escrever a resposta (testado no Windows 11: `ABM_GETTASKBARPOS` voltava
/// 0,0,0,0 e o `ABM_QUERYPOS` sem ajuste). Aí o RAD fala direto com a barra do
/// Explorer, do jeito que o shell32 fala com uma bandeja de outro processo.
pub fn appbar_msg(msg: u32, d: &mut APPBARDATA) -> usize {
    if own_tray_first() {
        if let Some(r) = appbar_direct(msg, d) {
            return r;
        }
    }
    unsafe { SHAppBarMessage(msg, d) }
}

/// A primeira `Shell_TrayWnd` do `FindWindow` é a bandeja do RAD?
fn own_tray_first() -> bool {
    unsafe {
        let Ok(h) = FindWindowW(w!("Shell_TrayWnd"), None) else { return false };
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

/// `APPBARDATA` como vai no pacote (handles em 32 bits, 40 bytes).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AppBarData3264 {
    cb_size: u32,
    hwnd: u32,
    cb: u32,
    edge: u32,
    rc: RECT,
    lparam: i64,
}

/// Pacote `WM_COPYDATA` (dwData 0) do `SHAppBarMessage`, 64 bytes: os dados,
/// a mensagem ABM_* e o bloco compartilhado onde o Explorer escreve a resposta.
#[repr(C)]
#[derive(Default)]
struct AppBarMsg3264 {
    abd: AppBarData3264,
    msg: u32,
    _pad: u32,
    shared: u64,
    pid: u32,
    _pad2: u32,
}

fn appbar_direct(msg: u32, d: &mut APPBARDATA) -> Option<usize> {
    let tray = tray()?;
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(tray, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let abd = AppBarData3264 {
            cb_size: std::mem::size_of::<AppBarData3264>() as u32,
            hwnd: d.hWnd.0 as isize as u32,
            cb: d.uCallbackMessage,
            edge: d.uEdge,
            rc: d.rc,
            lparam: d.lParam.0 as i64,
        };
        let size = std::mem::size_of::<AppBarData3264>() as u32;
        // bloco da resposta já no processo do Explorer
        let h = SHAllocShared(Some(&abd as *const _ as *const _), size, pid);
        if h.is_invalid() {
            return None;
        }
        let m = AppBarMsg3264 { abd, msg, shared: h.0 as usize as u64, pid, ..Default::default() };
        let cds = COPYDATASTRUCT {
            dwData: 0,
            cbData: std::mem::size_of::<AppBarMsg3264>() as u32,
            lpData: &m as *const _ as *mut _,
        };
        let mut res = 0usize;
        let ok = SendMessageTimeoutW(
            tray,
            WM_COPYDATA,
            WPARAM(d.hWnd.0 as usize),
            LPARAM(&cds as *const _ as isize),
            SMTO_ABORTIFHUNG,
            5000,
            Some(&mut res),
        );
        let p = SHLockShared(h, pid) as *const AppBarData3264;
        if !p.is_null() {
            let out = *p;
            let _ = SHUnlockShared(p as *const _);
            d.uEdge = out.edge;
            d.rc = out.rc;
        }
        let _ = SHFreeShared(h, pid);
        (ok.0 != 0).then_some(res)
    }
}

fn appbar_state() -> u32 {
    let mut d = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
    appbar_msg(ABM_GETSTATE, &mut d) as u32
}

fn set_appbar_state(state: u32) {
    let mut d = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: tray().unwrap_or_default(),
        lParam: LPARAM(state as isize),
        ..Default::default()
    };
    appbar_msg(ABM_SETSTATE, &mut d);
}

fn read_marker(file: &Path) -> Option<Marker> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// Restaura o que o arquivo de recuperação mandar e o apaga. Sem arquivo não
/// mexe em nada (a barra nunca foi escondida por este Island). Chamado ao
/// iniciar, pelo vigia e ao desligar o módulo/sair.
pub fn restore_from(file: &Path) {
    let Some(m) = read_marker(file) else {
        // arquivo ilegível: melhor mostrar a barra que deixá-la sumida
        if file.exists() {
            show_tray();
            let _ = std::fs::remove_file(file);
        }
        return;
    };
    if appbar_state() != m.state {
        set_appbar_state(m.state);
    }
    show_tray();
    let _ = std::fs::remove_file(file);
}

/// Quanto a barra principal reserva no pé da tela, em px físicos (0 se ela
/// está em auto-ocultar ou em outra borda). Escondida, ela continua reservando.
pub fn bottom_reserve() -> i32 {
    if appbar_state() & ABS_AUTOHIDE != 0 {
        return 0;
    }
    let mut d = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
    if appbar_msg(ABM_GETTASKBARPOS, &mut d) == 0 || d.uEdge != ABE_BOTTOM {
        return 0;
    }
    (d.rc.bottom - d.rc.top).max(0)
}

fn show_tray() {
    if let Some(t) = tray() {
        unsafe {
            let _ = ShowWindow(t, SW_SHOWNA);
        }
    }
}

/// Esconde a barra principal e mantém o vigia vivo enquanto ela estiver
/// escondida. Vive na thread da dock.
pub struct Hider {
    file: PathBuf,
    watchdog: Option<std::process::Child>,
    pub hidden: bool,
}

impl Hider {
    pub fn new(file: PathBuf) -> Self {
        Self { file, watchdog: None, hidden: false }
    }

    /// Esconde (pode chamar de novo, ex.: o Explorer reiniciou). Se não der
    /// para gravar o arquivo de recuperação, não esconde nada.
    pub fn hide(&mut self) -> bool {
        if !self.file.exists() {
            let m = Marker { state: appbar_state() };
            let ok = self.file.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok())
                && serde_json::to_string(&m).is_ok_and(|txt| std::fs::write(&self.file, txt).is_ok());
            if !ok {
                return false;
            }
        }
        if self.watchdog.is_none() {
            self.watchdog = spawn_watchdog(&self.file);
        }
        self.hide_window();
        self.hidden = true;
        true
    }

    /// Esconde só a janela (o Explorer a mostrou de novo). Assíncrono: a
    /// janela é de outro processo e não vale esperar por ele.
    pub fn hide_window(&self) {
        if let Some(t) = tray() {
            unsafe {
                let _ = ShowWindowAsync(t, SW_HIDE);
            }
        }
    }

    /// Devolve a barra como estava e encerra o vigia.
    pub fn restore(&mut self) {
        restore_from(&self.file);
        self.hidden = false;
        if let Some(mut c) = self.watchdog.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for Hider {
    fn drop(&mut self) {
        if self.hidden {
            self.restore();
        }
    }
}

fn spawn_watchdog(file: &Path) -> Option<std::process::Child> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().ok()?;
    std::process::Command::new(exe)
        .arg(WATCHDOG_ARG)
        .arg(std::process::id().to_string())
        .arg(file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000) // sem janela de console
        .spawn()
        .ok()
}

/// Corpo do processo vigia: `island.exe --taskbar-watchdog <pid> <arquivo>`.
/// Bloqueia até o Island `<pid>` terminar e então restaura a barra se o
/// arquivo de recuperação ainda existir (saída normal já o apagou).
/// Devolve `false` se o exe não foi chamado como vigia.
pub fn watchdog_main() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(WATCHDOG_ARG) {
        return false;
    }
    let pid = args.get(2).and_then(|s| s.parse::<u32>().ok());
    let file = args.get(3).map(PathBuf::from);
    if let (Some(pid), Some(file)) = (pid, file) {
        unsafe {
            // sem handle (o Island já morreu): restaura na hora
            if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
                WaitForSingleObject(h, INFINITE);
                let _ = CloseHandle(h);
            }
        }
        // arquivo ainda lá = o Island morreu com a dock ligada: além da barra,
        // os ícones da bandeja voltam a se registrar direto no Explorer
        let crashed = file.exists();
        restore_from(&file);
        if crashed {
            broadcast_taskbar_created();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::Marker;

    #[test]
    fn marcador_ida_e_volta() {
        let m = Marker { state: 2 };
        let txt = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Marker>(&txt).unwrap(), m);
        // arquivo truncado (queda no meio da gravação) não vira estado inventado
        assert!(serde_json::from_str::<Marker>("{\"sta").is_err());
    }
}
