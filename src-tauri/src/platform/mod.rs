//! Camada de plataforma. No Windows lê as APIs reais; em outros sistemas
//! usa dados simulados para o app rodar e ser testado.

use serde::Serialize;

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub app: String,
    pub playing: bool,
    /// posição em segundos no instante `updated_ms`
    pub position: f64,
    pub duration: f64,
    /// epoch em ms de quando `position` foi amostrada
    pub updated_ms: i64,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Network {
    pub online: bool,
    pub wifi: bool,
    pub name: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct WifiNet {
    pub ssid: String,
    /// 0..100
    pub signal: u32,
    pub secure: bool,
    /// existe perfil salvo (dá para conectar com um clique)
    pub saved: bool,
    pub connected: bool,
    pub profile: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub active: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PowerInfo {
    pub plans: Vec<PowerPlan>,
    /// "efficiency" | "balanced" | "performance" (modo de energia do Windows 11)
    pub mode: Option<String>,
    /// economia de energia ligada
    pub saver: bool,
    pub ac: bool,
    pub percent: Option<u8>,
    /// segundos restantes na bateria (quando o Windows estima)
    pub time_left: Option<u32>,
}

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(not(windows))]
mod mock;
#[cfg(not(windows))]
pub use mock::*;
