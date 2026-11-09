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
    pub brightness: bool,
    pub caps: bool,
    pub battery: bool,
    pub network: bool,
    pub track: bool,
    pub clipboard: bool,
    pub notifications: bool,
    pub pomo_sound: bool,
    /// dispositivo USB conectado
    pub usb: bool,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            volume: true,
            brightness: true,
            caps: true,
            battery: true,
            network: true,
            track: true,
            clipboard: true,
            notifications: true,
            pomo_sound: true,
            usb: true,
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
    /// "auto" (cor da capa), "system" (cor de destaque do Windows) ou "#rrggbb"
    pub color: String,
}

impl Default for Aura {
    fn default() -> Self {
        Self { intensity: "subtle".into(), expanded: false, color: "auto".into() }
    }
}

/// Onda de luz que sai da ilha ao conectar um dispositivo USB. A cor é a
/// mesma da aura (`Aura::color`); o aviso em si é o `Modules::usb`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct UsbWave {
    /// "off" | "subtle" | "medium"
    pub intensity: String,
    /// "fast" | "normal" | "slow" (1100 / 1700 / 2600 ms, no frontend)
    pub duration: String,
    /// distorção do vidro no anel: "off" | "light" | "strong" (0 / 9 / 18 px)
    pub distortion: String,
}

impl Default for UsbWave {
    fn default() -> Self {
        Self { intensity: "subtle".into(), duration: "fast".into(), distortion: "light".into() }
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

/// Barra de tarefas do Island (dock flutuante de vidro, `src/dock`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Taskbar {
    /// desenha a dock e esconde a barra nativa do Windows; desligada por
    /// padrão, porque mexe na barra do usuário
    pub enabled: bool,
    /// "system" (segue o tema claro/escuro dos apps) | "dark" | "light" | "clear"
    pub glass: String,
    /// posição dos ícones: "center" | "left"
    pub align: String,
    /// "small" | "medium" | "large" (ícones de 38, 46 e 54 px)
    pub size: String,
    /// ampliação ao passar o mouse: "off" | "subtle" | "strong"
    pub magnify: String,
    /// botão ⌃ que abre os ícones ocultos da bandeja
    pub tray_button: bool,
}

impl Default for Taskbar {
    fn default() -> Self {
        Self {
            enabled: false,
            glass: "system".into(),
            align: "center".into(),
            size: "medium".into(),
            magnify: "subtle".into(),
            tray_button: true,
        }
    }
}

impl Taskbar {
    /// Valor fora da lista (arquivo editado à mão, versão futura) volta ao padrão.
    fn sanitize(&mut self) {
        let d = Taskbar::default();
        let fix = |v: &mut String, ok: &[&str], def: &str| {
            if !ok.contains(&v.as_str()) {
                *v = def.into();
            }
        };
        fix(&mut self.glass, &["system", "dark", "light", "clear"], &d.glass);
        fix(&mut self.align, &["center", "left"], &d.align);
        fix(&mut self.size, &["small", "medium", "large"], &d.size);
        fix(&mut self.magnify, &["off", "subtle", "strong"], &d.magnify);
    }

    /// Lado do ícone em px CSS (o mesmo `--icon` de `ui/dock.css`).
    pub fn icon_px(&self) -> f64 {
        match self.size.as_str() {
            "small" => 38.0,
            "large" => 54.0,
            _ => 46.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub modules: Modules,
    pub aura: Aura,
    pub usb_wave: UsbWave,
    /// escala da ilha: 0.9 | 1.0 | 1.1
    pub scale: f64,
    /// nome do monitor (ex.: `\\.\DISPLAY2`); `None` = principal
    pub monitor: Option<String>,
    /// onde a ilha fica no monitor: borda + lado, um dos `POSITIONS`
    /// (ex.: "bottom-right"); na base ela abre para cima
    pub position: String,
    pub shortcuts: Shortcuts,
    /// apps cujas notificações não viram aviso na ilha (continuam na aba)
    pub muted_apps: Vec<String>,
    /// app cobrindo a ilha (tela cheia ou janela maximizada por baixo dela):
    /// "hide" (some) | "peek" (fica um traço no topo)
    pub fullscreen: String,
    /// esconde o aviso de volume do próprio Windows (só com o aviso `volume` ligado)
    pub hide_volume_osd: bool,
    /// barra de tarefas própria (dock flutuante)
    pub taskbar: Taskbar,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            modules: Modules::default(),
            aura: Aura::default(),
            usb_wave: UsbWave::default(),
            scale: 1.0,
            monitor: None,
            position: "top-center".into(),
            shortcuts: Shortcuts::default(),
            muted_apps: Vec::new(),
            fullscreen: "hide".into(),
            hide_volume_osd: true,
            taskbar: Taskbar::default(),
        }
    }
}

pub const SCALES: [f64; 3] = [0.9, 1.0, 1.1];

/// Posições da ilha: topo ou base, à esquerda, no centro ou à direita.
pub const POSITIONS: [&str; 6] = ["top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"];

/// `position` desmontada para quem posiciona a janela: (na base?, lado), com
/// lado -1 = esquerda, 0 = centro, 1 = direita. Espera um valor já corrigido
/// pelo `sanitize`; qualquer outro cai no topo centralizado.
pub fn anchor(position: &str) -> (bool, i32) {
    match position.split_once('-') {
        Some((v, h)) if POSITIONS.contains(&position) => {
            (v == "bottom", match h { "left" => -1, "right" => 1, _ => 0 })
        }
        _ => (false, 0),
    }
}

impl Settings {
    /// Corrige valores fora do esperado (arquivo editado à mão, versão antiga).
    pub fn sanitize(&mut self) {
        let s = self.scale;
        self.scale = SCALES
            .iter()
            .copied()
            .min_by(|a, b| (a - s).abs().total_cmp(&(b - s).abs()))
            .unwrap_or(1.0);
        // posição desconhecida (versão futura, arquivo editado à mão) volta ao padrão
        if !POSITIONS.contains(&self.position.as_str()) {
            self.position = "top-center".into();
        }
        if !["off", "subtle", "medium"].contains(&self.aura.intensity.as_str()) {
            self.aura.intensity = "subtle".into();
        }
        let c = &self.aura.color;
        let hex_ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
        if c != "auto" && c != "system" && !hex_ok {
            self.aura.color = "auto".into();
        }
        // onda do USB: valor desconhecido volta para o padrão daquela opção
        let w = &mut self.usb_wave;
        let d = UsbWave::default();
        if !["off", "subtle", "medium"].contains(&w.intensity.as_str()) {
            w.intensity = d.intensity;
        }
        if !["fast", "normal", "slow"].contains(&w.duration.as_str()) {
            w.duration = d.duration;
        }
        if !["off", "light", "strong"].contains(&w.distortion.as_str()) {
            w.distortion = d.distortion;
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
        self.taskbar.sanitize();
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

#[cfg(test)]
mod tests {
    use super::Settings;

    fn color_after_sanitize(c: &str) -> String {
        let mut s = Settings::default();
        s.aura.color = c.into();
        s.sanitize();
        s.aura.color
    }

    #[test]
    fn aura_color_valida() {
        assert_eq!(color_after_sanitize("auto"), "auto");
        assert_eq!(color_after_sanitize("system"), "system");
        assert_eq!(color_after_sanitize("#a94dc1"), "#a94dc1");
        // valor desconhecido (versão futura, arquivo editado à mão) volta para a capa
        assert_eq!(color_after_sanitize("System"), "auto");
        assert_eq!(color_after_sanitize("#12345"), "auto");
    }

    #[test]
    fn config_antiga_sem_cor_continua_auto() {
        let s: Settings = serde_json::from_str(r#"{"aura":{"intensity":"medium"}}"#).unwrap();
        assert_eq!(s.aura.color, "auto");
    }

    #[test]
    fn config_antiga_sem_posicao_fica_no_topo_centralizada() {
        let mut s: Settings = serde_json::from_str(r#"{"scale":1.1,"monitor":"\\\\.\\DISPLAY2"}"#).unwrap();
        s.sanitize();
        assert_eq!(s.position, "top-center");
        assert_eq!(s.monitor.as_deref(), Some(r"\\.\DISPLAY2"));
    }

    #[test]
    fn posicao_invalida_volta_ao_topo_centralizada() {
        for bad in ["", "bottom", "Bottom-Left", "middle-center", "bottom-centre", " top-left"] {
            let mut s = Settings::default();
            s.position = bad.into();
            s.sanitize();
            assert_eq!(s.position, "top-center", "{bad:?}");
        }
        // as seis válidas ficam como estão
        for ok in super::POSITIONS {
            let mut s = Settings::default();
            s.position = ok.into();
            s.sanitize();
            assert_eq!(s.position, ok);
        }
    }

    #[test]
    fn ancora_da_posicao() {
        use super::anchor;
        assert_eq!(anchor("top-center"), (false, 0));
        assert_eq!(anchor("top-left"), (false, -1));
        assert_eq!(anchor("top-right"), (false, 1));
        assert_eq!(anchor("bottom-left"), (true, -1));
        assert_eq!(anchor("bottom-center"), (true, 0));
        assert_eq!(anchor("bottom-right"), (true, 1));
        // sem passar pelo sanitize, um valor estranho não vira base nem lado
        assert_eq!(anchor("bottom-middle"), (false, 0));
        assert_eq!(anchor("bottom"), (false, 0));
    }

    #[test]
    fn config_antiga_sem_usb_ganha_os_padroes() {
        // arquivo de antes da onda do USB: sem `usb_wave` e sem `modules.usb`
        let mut s: Settings = serde_json::from_str(r#"{"modules":{"caps":false},"aura":{"intensity":"medium"}}"#).unwrap();
        s.sanitize();
        assert!(s.modules.usb);
        assert!(!s.modules.caps);
        assert_eq!(s.usb_wave.intensity, "subtle");
        assert_eq!(s.usb_wave.duration, "fast");
        assert_eq!(s.usb_wave.distortion, "light");
    }

    #[test]
    fn onda_usb_parcial_completa_com_o_padrao() {
        let s: Settings = serde_json::from_str(r#"{"usb_wave":{"duration":"slow"}}"#).unwrap();
        assert_eq!(s.usb_wave.intensity, "subtle");
        assert_eq!(s.usb_wave.duration, "slow");
        assert_eq!(s.usb_wave.distortion, "light");
    }

    #[test]
    fn onda_usb_invalida_volta_ao_padrao() {
        let mut s = Settings::default();
        s.usb_wave.intensity = "forte".into();
        s.usb_wave.duration = "1700".into();
        s.usb_wave.distortion = "Strong".into();
        s.sanitize();
        assert_eq!(s.usb_wave.intensity, "subtle");
        assert_eq!(s.usb_wave.duration, "fast");
        assert_eq!(s.usb_wave.distortion, "light");
        // valores válidos ficam como estão
        s.usb_wave.intensity = "off".into();
        s.usb_wave.duration = "slow".into();
        s.usb_wave.distortion = "strong".into();
        s.sanitize();
        assert_eq!(s.usb_wave.intensity, "off");
        assert_eq!(s.usb_wave.duration, "slow");
        assert_eq!(s.usb_wave.distortion, "strong");
    }

    #[test]
    fn config_antiga_sem_barra_de_tarefas_usa_o_padrao() {
        let s: Settings = serde_json::from_str(r#"{"scale":1.1,"aura":{"intensity":"medium"}}"#).unwrap();
        let t = &s.taskbar;
        // desligada: ligar esconde a barra do Windows, então só por escolha do usuário
        assert!(!t.enabled);
        assert_eq!((t.glass.as_str(), t.align.as_str(), t.size.as_str(), t.magnify.as_str()), ("system", "center", "medium", "subtle"));
        assert!(t.tray_button);
        // bloco parcial: o que falta ganha o padrão
        let s: Settings = serde_json::from_str(r#"{"taskbar":{"enabled":true,"size":"large"}}"#).unwrap();
        assert!(s.taskbar.enabled);
        assert_eq!(s.taskbar.size, "large");
        assert_eq!(s.taskbar.glass, "system");
        assert!(s.taskbar.tray_button);
    }

    #[test]
    fn barra_de_tarefas_valores_invalidos_voltam_ao_padrao() {
        let mut s = Settings::default();
        s.taskbar.glass = "Dark".into();
        s.taskbar.align = "right".into();
        s.taskbar.size = "46".into();
        s.taskbar.magnify = "max".into();
        s.sanitize();
        assert_eq!(s.taskbar, super::Taskbar::default());
        for (v, px) in [("small", 38.0), ("medium", 46.0), ("large", 54.0)] {
            s.taskbar.size = v.into();
            s.sanitize();
            assert_eq!(s.taskbar.icon_px(), px);
        }
        s.taskbar.glass = "clear".into();
        s.sanitize();
        assert_eq!(s.taskbar.glass, "clear");
    }
}
