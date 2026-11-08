//! Configurações do usuário, em `%APPDATA%\dev.r4nges.island\settings.json`.
//!
//! O Rust é a fonte da verdade: lê no início, entrega ao frontend
//! (`get_settings`) e grava o que o painel manda (`set_settings`). Campos
//! ausentes no arquivo assumem o padrão (`#[serde(default)]`), então versões
//! novas podem acrescentar opções sem quebrar o arquivo de quem atualizou.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Avisos (HUDs/toasts) que o usuário pode desligar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Modules {
    pub volume: bool,
    pub caps: bool,
    pub battery: bool,
    pub network: bool,
    pub track: bool,
    pub clipboard: bool,
    pub notifications: bool,
    pub pomo_sound: bool,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            volume: true,
            caps: true,
            battery: true,
            network: true,
            track: true,
            clipboard: true,
            notifications: true,
            pomo_sound: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Aura {
    /// "off" | "subtle" | "medium"
    pub intensity: String,
    /// aura também com a ilha expandida, mesmo sem mídia
    pub expanded: bool,
    /// "auto" (cor da capa) ou "#rrggbb"
    pub color: String,
}

impl Default for Aura {
    fn default() -> Self {
        Self { intensity: "subtle".into(), expanded: false, color: "auto".into() }
    }
}

/// Atalhos globais no formato do global-hotkey (`Ctrl+Alt+KeyI`); vazio = desligado.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Shortcuts {
    /// abrir/fechar a ilha
    pub open: String,
    /// tocar/pausar a mídia
    pub play: String,
    /// iniciar/pausar o pomodoro
    pub pomo: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self { open: "Ctrl+Alt+KeyI".into(), play: "Ctrl+Alt+Space".into(), pomo: "Ctrl+Alt+KeyP".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub modules: Modules,
    pub aura: Aura,
    /// escala da ilha: 0.9 | 1.0 | 1.1
    pub scale: f64,
    /// nome do monitor (ex.: `\\.\DISPLAY2`); `None` = principal
    pub monitor: Option<String>,
    pub shortcuts: Shortcuts,
    /// apps cujas notificações não viram aviso na ilha (continuam na aba)
    pub muted_apps: Vec<String>,
    /// app em tela cheia: "hide" (some) | "peek" (fica um traço no topo)
    pub fullscreen: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            modules: Modules::default(),
            aura: Aura::default(),
            scale: 1.0,
            monitor: None,
            shortcuts: Shortcuts::default(),
            muted_apps: Vec::new(),
            fullscreen: "hide".into(),
        }
    }
}

pub const SCALES: [f64; 3] = [0.9, 1.0, 1.1];

impl Settings {
    /// Corrige valores fora do esperado (arquivo editado à mão, versão antiga).
    pub fn sanitize(&mut self) {
        let s = self.scale;
        self.scale = SCALES
            .iter()
            .copied()
            .min_by(|a, b| (a - s).abs().total_cmp(&(b - s).abs()))
            .unwrap_or(1.0);
        if !["off", "subtle", "medium"].contains(&self.aura.intensity.as_str()) {
            self.aura.intensity = "subtle".into();
        }
        let c = &self.aura.color;
        let hex_ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
        if c != "auto" && !hex_ok {
            self.aura.color = "auto".into();
        }
        for s in [&mut self.shortcuts.open, &mut self.shortcuts.play, &mut self.shortcuts.pomo] {
            *s = s.trim().to_string();
        }
        self.muted_apps.retain(|a| !a.trim().is_empty());
        self.muted_apps.sort();
        self.muted_apps.dedup();
        if self.fullscreen != "peek" {
            self.fullscreen = "hide".into();
        }
    }
}

pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(file: &Path) -> Settings {
    let mut s: Settings = std::fs::read_to_string(file)
        .ok()
        .and_then(|txt| serde_json::from_str(&txt).ok())
        .unwrap_or_default();
    s.sanitize();
    s
}

/// Grava num temporário e renomeia: uma queda no meio não corrompe o arquivo.
pub fn save(file: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    let txt = serde_json::to_string_pretty(s).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, txt)?;
    std::fs::rename(&tmp, file)
}
