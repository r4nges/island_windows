//! Atualização automática via tauri-plugin-updater.
//!
//! Checa o `latest.json` do último Release ao iniciar e a cada 6 h. Quando
//! há versão nova, avisa o frontend (`island:update`) e troca o item da
//! bandeja para "Instalar". Instalar baixa o instalador NSIS, confere a
//! assinatura com a chave pública do tauri.conf.json e roda o instalador em
//! modo passivo; o hook do NSIS re-registra o pacote esparso e o app reabre.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_updater::{Update, UpdaterExt};

const EVERY: Duration = Duration::from_secs(6 * 3600);
const RETRY: Duration = Duration::from_secs(30 * 60);
/// espera inicial: no boot a rede costuma demorar a subir
const FIRST: Duration = Duration::from_secs(20);

#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
    installing: AtomicBool,
    tray_item: Mutex<Option<MenuItem<Wry>>>,
}

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    version: String,
    current: String,
    notes: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "state", rename_all = "lowercase")]
enum Progress {
    Downloading { pct: Option<u8> },
    Installing,
    Error { msg: String },
}

/// Guarda o item "Procurar atualizações" da bandeja para trocar o texto depois.
pub fn set_tray_item(app: &AppHandle, item: MenuItem<Wry>) {
    *app.state::<Updates>().tray_item.lock().unwrap() = Some(item);
}

fn set_tray_text(app: &AppHandle, text: &str) {
    if let Some(item) = app.state::<Updates>().tray_item.lock().unwrap().as_ref() {
        let _ = item.set_text(text);
    }
}

/// Consulta o servidor. `Some` = há versão nova (já guardada como pendente).
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let info = found.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone().filter(|b| !b.trim().is_empty()),
    });
    *app.state::<Updates>().pending.lock().unwrap() = found;
    match &info {
        Some(i) => {
            set_tray_text(app, &format!("Instalar atualização v{}", i.version));
            let _ = app.emit("island:update", i);
        }
        None => set_tray_text(app, "Procurar atualizações"),
    }
    Ok(info)
}

/// Thread de checagem periódica. Em build de debug não roda sozinha
/// (só pela bandeja), para o `npm run dev` não oferecer o instalador.
pub fn spawn(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    thread::spawn(move || {
        thread::sleep(FIRST);
        loop {
            let wait = match tauri::async_runtime::block_on(check(&app)) {
                Ok(_) => EVERY,
                Err(_) => RETRY,
            };
            thread::sleep(wait);
        }
    });
}

/// Clique no item da bandeja: instala se já houver pendente; senão procura
/// e avisa o resultado (inclusive "já está na última versão").
pub fn tray_clicked(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if app.state::<Updates>().pending.lock().unwrap().is_some() {
            let _ = install(app).await;
            return;
        }
        set_tray_text(&app, "Procurando…");
        match check(&app).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = app.emit("island:update-none", env!("CARGO_PKG_VERSION"));
            }
            Err(e) => {
                set_tray_text(&app, "Procurar atualizações");
                let _ = app.emit("island:update-progress", Progress::Error { msg: e });
            }
        }
    });
}

#[tauri::command]
pub fn pending_update(state: tauri::State<'_, Updates>) -> Option<UpdateInfo> {
    state.pending.lock().unwrap().as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone().filter(|b| !b.trim().is_empty()),
    })
}

/// Baixa e instala. No Windows o plugin encerra o processo ao abrir o
/// instalador, então em caso de sucesso esta função não retorna.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    install(app).await
}

async fn install(app: AppHandle) -> Result<(), String> {
    let st = app.state::<Updates>();
    let Some(upd) = st.pending.lock().unwrap().clone() else {
        return Err("nenhuma atualização pendente".into());
    };
    if st.installing.swap(true, Ordering::SeqCst) {
        return Err("atualização já em andamento".into());
    }
    set_tray_text(&app, "Baixando atualização…");

    let (h1, h2) = (app.clone(), app.clone());
    let mut got: u64 = 0;
    let mut last: Option<Option<u8>> = None;
    let res = upd
        .download_and_install(
            move |chunk, total| {
                got += chunk as u64;
                let pct = total.filter(|t| *t > 0).map(|t| (got * 100 / t).min(100) as u8);
                if last != Some(pct) {
                    last = Some(pct);
                    let _ = h1.emit("island:update-progress", Progress::Downloading { pct });
                }
            },
            move || {
                let _ = h2.emit("island:update-progress", Progress::Installing);
            },
        )
        .await;

    // só chega aqui em erro (ou fora do Windows)
    st.installing.store(false, Ordering::SeqCst);
    match res {
        Ok(()) => {
            app.restart();
        }
        Err(e) => {
            let msg = e.to_string();
            set_tray_text(&app, &format!("Instalar atualização v{}", upd.version));
            let _ = app.emit("island:update-progress", Progress::Error { msg: msg.clone() });
            Err(msg)
        }
    }
}
