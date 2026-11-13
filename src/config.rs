use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::action::{Action, ScreenshotMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub layouts: Layouts,
    pub mac: Mac,
    pub bindings: Vec<Binding>,
}

/// ตั้งค่า "โหมด Mac"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Mac {
    /// เปิด/ปิดโหมด Mac — ปุ่ม `Alt` ฝั่งซ้ายจะทำหน้าที่เป็น `Ctrl` (⌘ ของ macOS)
    ///
    /// เมื่อเปิด: Alt+C = ⌘C (copy), Alt+Space = สลับภาษา, Alt+Tab = สลับแอป,
    /// Alt+Shift+3 = จับภาพ — เหมือนกด ⌘ บน macOS
    pub enabled: bool,
}

impl Default for Mac {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// เปิด/ปิดการทำงานทั้งหมด
    pub enabled: bool,
    /// ระดับ log: error | warn | info | debug | trace
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Layouts {
    /// รายการภาษาที่ใช้สลับไปมา เช่น ["en-US", "th-TH"]
    pub cycle: Vec<String>,
    /// ภาษาที่ต้องการตอนเริ่มโปรแกรม (ถ้าเว้นว่างจะไม่บังคับ)
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// คีย์ลัด เช่น "alt+space"
    pub keys: String,
    pub action: Action,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            layouts: Layouts::default(),
            mac: Mac::default(),
            bindings: default_bindings(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".into(),
        }
    }
}

impl Default for Layouts {
    fn default() -> Self {
        Self {
            cycle: vec!["en-US".into(), "th-TH".into()],
            default: Some("en-US".into()),
        }
    }
}

fn default_bindings() -> Vec<Binding> {
    vec![
        Binding {
            keys: "alt+space".into(),
            action: Action::SwitchLayout("cycle".into()),
            enabled: true,
        },
        Binding {
            keys: "ctrl+space".into(),
            action: Action::SwitchLayout("cycle".into()),
            enabled: true,
        },
        Binding {
            keys: "ctrl+shift+3".into(),
            action: Action::Screenshot(ScreenshotMode::Full),
            enabled: true,
        },
        Binding {
            keys: "ctrl+shift+4".into(),
            action: Action::Screenshot(ScreenshotMode::Region),
            enabled: true,
        },
        Binding {
            keys: "ctrl+shift+5".into(),
            action: Action::Screenshot(ScreenshotMode::Panel),
            enabled: true,
        },
        Binding {
            keys: "ctrl+tab".into(),
            action: Action::AppSwitcher,
            enabled: true,
        },
        // ⌘Q บน macOS = ปิดแอป (Windows ไม่มี Ctrl+Q → ใช้ Alt+F4 แทน)
        Binding {
            keys: "ctrl+q".into(),
            action: Action::SendKeys("alt+f4".into()),
            enabled: true,
        },
        // ⌘` บน macOS = สลับหน้าต่างของแอปเดียวกัน (Windows ใช้ Alt+`)
        Binding {
            keys: "ctrl+`".into(),
            action: Action::SendKeys("alt+`".into()),
            enabled: true,
        },
        // ⌘H บน macOS = ซ่อนแอป — Windows ไม่มีคีย์ตรง ๆ ใช้ Win+Down (ย่อหน้าต่างที่ active)
        Binding {
            keys: "ctrl+h".into(),
            action: Action::SendKeys("win+down".into()),
            enabled: true,
        },
    ]
}

/// %APPDATA%\MacKeys\config.toml
pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MacKeys")
        .join("config.toml")
}

pub fn dir_of(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("อ่านไฟล์ config ไม่ได้: {}", path.display()))?;
    let cfg: Config = toml::from_str(&text)
        .with_context(|| format!("แปลง config ไม่ได้ (TOML ผิดรูปแบบ?): {}", path.display()))?;
    Ok(cfg)
}

pub fn save(path: &Path, cfg: &Config) -> Result<()> {
    let dir = dir_of(path);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("สร้างโฟลเดอร์ config ไม่ได้: {}", dir.display()))?;
    let text = toml::to_string_pretty(cfg).context("แปลง config เป็น TOML ไม่ได้")?;
    std::fs::write(path, text).with_context(|| format!("บันทึก config ไม่ได้: {}", path.display()))?;
    Ok(())
}

/// สร้างไฟล์ config เริ่มต้นถ้ายังไม่มี
pub fn ensure_exists(path: &Path) -> Result<Config> {
    if path.exists() {
        return load(path);
    }
    let cfg = Config::default();
    save(path, &cfg)?;
    Ok(cfg)
}
