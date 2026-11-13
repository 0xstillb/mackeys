use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "mackeys",
    version,
    about = "Mac-style keyboard shortcuts for Windows",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// path ของไฟล์ config.toml
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// รันในโหมดปกติ (system tray + global hotkeys)
    Run,
    /// ตรวจสอบว่าคีย์ลัดไหนลงทะเบียนได้/ชนกับโปรแกรมอื่น
    Diagnose,
    /// แสดงสถานะปุ่ม modifier ที่กดอยู่ตอนนี้ (ใช้ตอนอาการ "พิมพ์ไม่ได้")
    DiagnoseKeys,
    /// แสดงรายการ keyboard layout ที่ติดตั้งในเครื่อง
    ListLayouts,
    /// ตั้งค่าให้โปรแกรมรันพร้อม Windows
    InstallAutostart,
    /// ยกเลิกการรันพร้อม Windows
    UninstallAutostart,
    /// เปิดโฟลเดอร์ที่เก็บไฟล์ config
    OpenConfigDir,
    /// ทดสอบการจับภาพหน้าจอ (บันทึกไฟล์ + คัดลอก clipboard) แล้วออก
    Capture,
    /// บันทึกภาพไอคอนของโปรแกรมเป็น PNG (ตรวจว่าไอคอนวาดถูกต้องไหม)
    IconPreview,
}
