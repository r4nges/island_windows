//! Atalhos globais (tauri-plugin-global-shortcut).
//!
//! As combinações ficam em `settings.shortcuts` no formato do `global-hotkey`
//! (`Ctrl+Alt+KeyI`), que aceita o próprio `KeyboardEvent.code` do navegador:
//! o painel grava o que o usuário apertou sem tabela de conversão. Ao disparar,
//! o Rust só emite `island:shortcut` com o nome da ação; o frontend executa.

use crate::settings::Shortcuts;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Default)]
pub struct Bindings {
    /// id do atalho registrado -> ação ("open" | "play" | "pomo")
    active: Mutex<Vec<(u32, &'static str)>>,
    /// por ação, o motivo de não ter registrado (ex.: combinação de outro app)
    errors: Mutex<HashMap<String, String>>,
}

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, sc, ev| {
            if ev.state != ShortcutState::Pressed {
                return;
            }
            let action = app
                .state::<Bindings>()
                .active
                .lock()
                .unwrap()
                .iter()
                .find(|(id, _)| *id == sc.id())
                .map(|(_, a)| *a);
            if let Some(a) = action {
                let _ = app.emit("island:shortcut", a);
            }
        })
        .build()
}

/// Solta tudo (enquanto o painel grava uma combinação nova).
pub fn pause(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
    app.state::<Bindings>().active.lock().unwrap().clear();
}

/// (Re)registra os atalhos das configurações.
pub fn apply(app: &AppHandle, s: &Shortcuts) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut active = Vec::new();
    let mut errors = HashMap::new();
    for (action, acc) in [("open", &s.open), ("play", &s.play), ("pomo", &s.pomo)] {
        if acc.trim().is_empty() {
            continue; // desligado
        }
        let res = acc
            .parse::<Shortcut>()
            .map_err(|e| e.to_string())
            .and_then(|sc| gs.register(sc).map(|_| sc.id()).map_err(|e| e.to_string()));
        match res {
            Ok(id) => active.push((id, action)),
            Err(e) => {
                errors.insert(action.to_string(), e);
            }
        }
    }
    let st = app.state::<Bindings>();
    *st.active.lock().unwrap() = active;
    *st.errors.lock().unwrap() = errors;
}

#[tauri::command]
pub fn shortcut_errors(state: tauri::State<'_, Bindings>) -> HashMap<String, String> {
    state.errors.lock().unwrap().clone()
}
