use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotMode {
    /// จับทั้งจอ → บันทึกไฟล์ + คัดลอก clipboard
    Full,
    /// จับเฉพาะบริเวณที่ลากเลือก (ส่งต่อให้ Snipping Tool ของ Windows)
    Region,
    /// เปิดหน้าต่าง Snipping Tool
    Panel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// สลับภาษาอินพุต — ค่า: "cycle" | "next" | ชื่อภาษา เช่น "th-TH" / "en-US"
    SwitchLayout(String),
    /// จับภาพหน้าจอ
    Screenshot(ScreenshotMode),
    /// สลับหน้าต่าง (เทียบ Cmd+Tab)
    AppSwitcher,
    /// ส่งชุดคีย์ไปยังหน้าต่างที่ focus อยู่ เช่น "alt+f4" (ใช้ทำ ⌘Q ที่ Windows ไม่มี)
    SendKeys(String),
}

impl Action {
    /// ข้อความอธิบายสั้น ๆ สำหรับแสดงผลใน GUI
    pub fn describe(&self) -> String {
        match self {
            Action::SwitchLayout(target) => match target.as_str() {
                "cycle" => "สลับภาษาตามรายการ".to_string(),
                "next" => "สลับไป layout ถัดไป".to_string(),
                other => format!("สลับไป {other}"),
            },
            Action::Screenshot(ScreenshotMode::Full) => "จับภาพทั้งจอ".to_string(),
            Action::Screenshot(ScreenshotMode::Region) => "จับภาพบริเวณ".to_string(),
            Action::Screenshot(ScreenshotMode::Panel) => "เปิด Snipping Tool".to_string(),
            Action::AppSwitcher => "สลับหน้าต่าง".to_string(),
            Action::SendKeys(target) => format!("ส่งคีย์ {target}"),
        }
    }
}
