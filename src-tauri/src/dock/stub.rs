//! Fora do Windows não há barra nativa para trocar: a dock nunca liga, mas a
//! API existe para o app compilar (como o `platform/mock.rs`).

use super::{ItemOut, TrayIconOut};
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Src;

pub struct Engine {
    pub h: Handle,
}

#[derive(Clone)]
pub struct Handle;

impl Engine {
    pub fn start(_app: AppHandle, _marker: PathBuf, _appbar: (i32, i32, i32, i32), _on_display: Box<dyn Fn() + Send>) -> Option<Engine> {
        None
    }
    pub fn stop(self) {}
}

impl Handle {
    pub fn set_glass(&self, _g: Option<(i32, i32, i32, i32)>) {}
    pub fn set_visible(&self, _v: bool) {}
    pub fn set_appbar(&self, _r: (i32, i32, i32, i32)) {}
    pub fn set_square(&self, _v: bool) {}
    pub fn max(&self) -> bool {
        false
    }
    pub fn items(&self) -> Vec<ItemOut> {
        Vec::new()
    }
    pub fn light(&self) -> bool {
        false
    }
    pub fn icon_srcs(&self, _key: &str) -> Option<Vec<Src>> {
        None
    }
    pub fn click(&self, _key: &str) {}
}

pub fn action(_kind: &str) {}

pub fn trash_src() -> Src {
    Src
}

pub fn icon(_srcs: Vec<Src>) -> Option<String> {
    None
}

pub fn cursor() -> Option<(i32, i32)> {
    None
}

/// Sem `Shell_TrayWnd` para hospedar: a bandeja própria nunca sobe.
pub struct TrayHost;

impl TrayHost {
    pub fn start(_on_change: Box<dyn Fn() + Send>) -> Option<TrayHost> {
        None
    }
    pub fn stop(self) {}
}

pub fn set_tray_anchor(_r: Option<(i32, i32, i32, i32)>) {}

pub fn tray_icons() -> Vec<TrayIconOut> {
    Vec::new()
}

pub fn tray_click(_id: &str, _how: &str) {}
