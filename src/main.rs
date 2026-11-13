#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod autostart;
mod cli;
mod config;
mod engine;
mod gui;
mod hotkey;
mod logging;
mod runner;
mod singleton;
mod tray;
mod ui;
mod win;

use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::engine::{Runtime, WinCommand};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Run);
    let config_path = cli.config.clone().unwrap_or_else(config::default_path);

    let result = match command {
        Command::Run => run_app(&config_path),
        Command::Diagnose => cmd_diagnose(&config_path),
        Command::DiagnoseKeys => cmd_diagnose_keys(),
        Command::ListLayouts => cmd_list_layouts(),
        Command::InstallAutostart => cmd_autostart(true),
        Command::UninstallAutostart => cmd_autostart(false),
        Command::OpenConfigDir => cmd_open_config(&config_path),
        Command::Capture => cmd_capture(),
        Command::IconPreview => cmd_icon_preview(),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✖ {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run_app(config_path: &Path) -> Result<()> {
    let cfg = config::ensure_exists(config_path)?;
    logging::init(&cfg.general.log_level);

    let _guard = singleton::InstanceGuard::acquire("Local\\MacKeys.SingleInstance")
        .ok_or_else(|| anyhow::anyhow!("โปรแกรมกำลังรันอยู่แล้ว (ดูไอคอนใน system tray)"))?;

    let (event_tx, event_rx) = mpsc::channel();
    let (action_tx, action_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();

    let rt = Arc::new(Runtime {
        config: RwLock::new(cfg),
        status: RwLock::new(Vec::new()),
        events: event_tx,
        actions: action_tx,
    });

    // worker thread สำหรับ action ที่ใช้เวลานาน เช่น จับภาพ
    let worker_rt = rt.clone();
    std::thread::spawn(move || {
        while let Ok(action) = action_rx.recv() {
            let rt = worker_rt.clone();
            // กันไม่ให้ action ที่พังลากทั้งโปรแกรมตาย
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                engine::execute(&rt, &action);
            }));
            if result.is_err() {
                tracing::error!("action {action:?} ล้มเหลว");
            }
        }
    });

    runner::spawn(rt.clone(), cmd_rx, config_path.to_path_buf());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MacKeys — ตั้งค่า")
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([760.0, 520.0])
            .with_visible(false),
        ..Default::default()
    };

    let app_rt = rt.clone();
    let app_cmd = cmd_tx.clone();
    let app_path = config_path.to_path_buf();

    eframe::run_native(
        "MacKeys",
        native_options,
        Box::new(move |cc| {
            gui::setup_fonts(&cc.egui_ctx);
            gui::init_context(cc.egui_ctx.clone());
            Ok(Box::new(gui::SettingsApp::new(
                app_rt,
                app_cmd,
                event_rx,
                app_path,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("เปิดหน้าต่างตั้งค่าไม่สำเร็จ: {e}"))?;

    let _ = cmd_tx.send(WinCommand::Quit);
    Ok(())
}

fn cmd_diagnose_keys() -> Result<()> {
    win::util::attach_parent_console();
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    let down = |vk: i32| unsafe { (GetAsyncKeyState(vk) as i32) & 0x8000 != 0 };

    let keys: [(i32, &str); 12] = [
        (0xA0, "LShift"),
        (0xA1, "RShift"),
        (0xA2, "LCtrl"),
        (0xA3, "RCtrl"),
        (0xA4, "LAlt"),
        (0xA5, "RAlt"),
        (0x5B, "LWin"),
        (0x5C, "RWin"),
        (0x20, "Space"),
        (0x09, "Tab"),
        (0x41, "A"),
        (0x1B, "Esc"),
    ];

    println!("สถานะปุ่มที่กดอยู่ตอนนี้:");
    let mut stuck = Vec::new();
    for (vk, name) in keys {
        if down(vk) {
            println!("  {name:<8} = กดค้างอยู่  ← ผิดปกติ");
            if !matches!(name, "A" | "Space" | "Tab" | "Esc") {
                stuck.push(name);
            }
        }
    }
    if stuck.is_empty() {
        println!("  (ไม่มีปุ่ม modifier ค้างอยู่ — ปกติ)");
    } else {
        println!();
        println!("⚠ พบปุ่มที่ค้าง: {}", stuck.join(", "));
        println!("  → พิมพ์ไม่ได้ / Alt+Space ใช้ไม่ได้ มักเกิดจากปุ่มค้างแบบนี้");
        println!("  → แก้ได้ทันที: คลิกขวาที่ไอคอน MacKeys ใน system tray");
        println!("     แล้วเลือก \"รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)\"");
    }
    Ok(())
}

fn cmd_diagnose(config_path: &Path) -> Result<()> {
    win::util::attach_parent_console();
    let cfg = config::load(config_path)?;

    // ถ้ามี instance ของ MacKeys กำลังรันอยู่ คีย์ลัดทั้งหมดจะถูกจองโดยตัวเอง
    // ต้องบอกผู้ใช้ให้ชัด ไม่งั้นจะเข้าใจผิดว่าชนกับโปรแกรมอื่น
    let self_running = singleton::is_running("Local\\MacKeys.SingleInstance");

    println!("ไฟล์ config : {}", config_path.display());
    println!("ตำแหน่ง exe : {}", std::env::current_exe()?.display());
    println!();

    if self_running {
        println!("⚠ MacKeys กำลังรันอยู่อยู่ → คีย์ลัดถูกจองโดยตัวเอง");
        println!("  ปิดโปรแกรมใน system tray ก่อน แล้วรัน diagnose ใหม่");
        println!("  เพื่อดูว่าคีย์ลัดไหนชนกับโปรแกรมอื่นจริงๆ");
        println!();
    }

    println!("คีย์ลัด:");
    for st in runner::diagnose(&cfg) {
        let status = if st.ok { "พร้อมใช้" } else { "ไม่ได้  " };
        let note = st.message.unwrap_or_default();
        println!("  {:<20} {status}  {note}", st.label);
    }

    println!();
    println!("keyboard layout ที่ติดตั้ง:");
    for l in win::layout::installed() {
        let current = win::layout::current().map(|c| c.lang_id == l.lang_id).unwrap_or(false);
        println!(
            "  {:<12} ({:04X}){}",
            l.name,
            l.lang_id,
            if current { "  ← กำลังใช้" } else { "" }
        );
    }

    println!();
    println!(
        "โหมด Mac (Alt = ⌘): {}",
        if cfg.mac.enabled {
            "เปิดอยู่"
        } else {
            "ปิดอยู่"
        }
    );
    println!(
        "รันพร้อม Windows: {}",
        if autostart::is_installed() { "เปิดอยู่" } else { "ปิดอยู่" }
    );

    Ok(())
}

fn cmd_list_layouts() -> Result<()> {
    win::util::attach_parent_console();
    let current = win::layout::current().map(|c| c.lang_id);
    println!("keyboard layout ที่ติดตั้ง:");
    for l in win::layout::installed() {
        println!(
            "  {:<12} ({:04X}){}",
            l.name,
            l.lang_id,
            if Some(l.lang_id) == current {
                "  ← กำลังใช้"
            } else {
                ""
            }
        );
    }
    Ok(())
}

fn cmd_autostart(install: bool) -> Result<()> {
    win::util::attach_parent_console();
    if install {
        autostart::install()?;
        println!("ตั้งค่ารันพร้อม Windows แล้ว");
    } else {
        autostart::uninstall()?;
        println!("ยกเลิกการรันพร้อม Windows แล้ว");
    }
    Ok(())
}

fn cmd_open_config(config_path: &Path) -> Result<()> {
    win::util::attach_parent_console();
    let dir = config::dir_of(config_path);
    std::fs::create_dir_all(&dir)?;
    win::util::open_folder(&dir)?;
    Ok(())
}

fn cmd_capture() -> Result<()> {
    win::util::attach_parent_console();
    let path = win::capture::capture_fullscreen()?;
    println!("บันทึกภาพแล้ว: {}", path.display());
    Ok(())
}

fn cmd_icon_preview() -> Result<()> {
    win::util::attach_parent_console();
    let path = std::env::temp_dir().join("mackeys-icon.png");
    win::icon::preview_png(&path)?;
    println!("บันทึกภาพไอคอนแล้ว: {}", path.display());
    Ok(())
}
