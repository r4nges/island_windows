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

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(not(windows))]
mod mock;
#[cfg(not(windows))]
pub use mock::*;
